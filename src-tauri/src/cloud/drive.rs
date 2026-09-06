//! Bounded Drive v3 API operations. Writes use caller-persisted preallocated IDs, never name overwrite.
use super::{
    crypto::{SpaceKey, MAX_ENCRYPTED},
    oauth::{http, AccessToken},
    Result,
};
use crate::sync::bundle::{token, Bundle, MAX_OBJECTS};
use reqwest::blocking::{Client, Response};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{io::Read, time::Duration};
#[derive(Clone, Copy)]
pub enum ObjectKind {
    Session,
    Device,
    Portable,
}
impl ObjectKind {
    fn mime(self) -> &'static str {
        match self {
            Self::Session => "application/octet-stream",
            Self::Device => "application/vnd.bastet.device-encrypted",
            Self::Portable => "application/vnd.bastet.portable-encrypted",
        }
    }
}
const FILES: &str = "https://www.googleapis.com/drive/v3/files";
const UPLOAD: &str = "https://www.googleapis.com/upload/drive/v3/files";
const FOLDER: &str = "application/vnd.google-apps.folder";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub permission_id: String,
    pub display_name: Option<String>,
    pub email_address: Option<String>,
}
impl Account {
    pub fn validate(&self) -> Result<()> {
        if !token(&self.permission_id)
            || [&self.display_name, &self.email_address].iter().any(|v| {
                v.as_ref()
                    .is_some_and(|s| s.len() > 1024 || s.chars().any(char::is_control))
            })
        {
            return Err("drive_invalid_account".into());
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct File {
    pub id: String,
    pub name: String,
    pub mime_type: String,
    #[serde(default)]
    pub parents: Vec<String>,
    #[serde(default)]
    pub trashed: bool,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub size: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Page {
    #[serde(default)]
    files: Vec<File>,
    next_page_token: Option<String>,
    #[serde(default)]
    incomplete_search: bool,
}
pub struct Drive {
    client: Client,
    token: AccessToken,
    files_url: String,
    upload_url: String,
    about_url: String,
}
fn id(id: &str) -> Result<()> {
    if token(id) {
        Ok(())
    } else {
        Err("invalid_drive_id".into())
    }
}
fn checked(response: Response) -> Result<Response> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status().as_u16();
    // Do not expose provider bodies, authorization headers, URLs or account details in errors.
    Err(match status {
        401 => "reauth_required",
        403 => "drive_forbidden",
        404 => "drive_not_found",
        409 => "drive_id_exists",
        429 => "drive_rate_limited",
        500..=599 => "drive_temporary",
        _ => "drive_request_failed",
    }
    .into())
}
fn read(response: Response, limit: usize) -> Result<Vec<u8>> {
    let response = checked(response)?;
    if response.content_length().is_some_and(|n| n > limit as u64) {
        return Err("drive_response_limit".into());
    }
    let mut bytes = Vec::new();
    if let Some(p) = crate::progress::current() {
        p.body(response.content_length());
    }
    super::traffic::download(response)
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "network_unavailable")?;
    if bytes.len() > limit {
        return Err("drive_response_limit".into());
    }
    Ok(bytes)
}
/// Scheduler guidance only. No hidden sleeping or automatic retry of non-idempotent writes.
/// A 403 may be permission loss; callers must not blindly retry it.
pub fn retry_delay(
    status: u16,
    attempt: u32,
    retry_after: Option<u64>,
    jitter_ms: u16,
) -> Option<Duration> {
    if attempt >= 5 || !(status == 429 || (500..=599).contains(&status)) {
        return None;
    }
    Some(Duration::from_millis(
        (retry_after.unwrap_or(1u64 << attempt).min(120) * 1000) + u64::from(jitter_ms.min(1000)),
    ))
}
impl Drive {
    pub fn is_connected(&self) -> bool {
        !self.token.expired()
    }
    pub fn new(token: AccessToken) -> Result<Self> {
        Ok(Self {
            client: http()?,
            token,
            files_url: FILES.into(),
            upload_url: UPLOAD.into(),
            about_url: "https://www.googleapis.com/drive/v3/about".into(),
        })
    }
    pub fn account(&self) -> Result<Account> {
        self.check_token()?;
        let r = self
            .client
            .get(&self.about_url)
            .bearer_auth(self.token.value.as_str())
            .query(&[("fields", "user(permissionId,displayName,emailAddress)")])
            .timeout(crate::resources::request_timeout())
            .send()
            .map_err(|_| "network_unavailable")?;
        #[derive(Deserialize)]
        struct About {
            user: Account,
        }
        let about: About =
            serde_json::from_slice(&read(r, 65536)?).map_err(|_| "drive_invalid_account")?;
        about.user.validate()?;
        Ok(about.user)
    }
    fn check_token(&self) -> Result<()> {
        if self.token.expired() {
            Err("reauth_required".into())
        } else {
            Ok(())
        }
    }
    /// Persist this ID in the local queue before creation; retry with exactly this same ID.
    pub fn allocate_id(&self) -> Result<String> {
        self.check_token()?;
        let r = self
            .client
            .get(format!("{}/generateIds", self.files_url))
            .bearer_auth(self.token.value.as_str())
            .query(&[("count", "1"), ("space", "drive"), ("type", "files")])
            .timeout(crate::resources::request_timeout())
            .send()
            .map_err(|_| "network_unavailable")?;
        #[derive(Deserialize)]
        struct Ids {
            ids: Vec<String>,
        }
        let mut ids: Ids =
            serde_json::from_slice(&read(r, 65536)?).map_err(|_| "drive_invalid_response")?;
        if ids.ids.len() != 1 {
            return Err("drive_invalid_response".into());
        }
        let v = ids.ids.remove(0);
        id(&v)?;
        Ok(v)
    }
    pub fn list_folders(&self) -> Result<Vec<File>> {
        self.list("trashed = false and mimeType = 'application/vnd.google-apps.folder'")
    }
    pub fn list_objects(&self, folder: &str) -> Result<Vec<File>> {
        id(folder)?;
        self.verify_folder(folder)?;
        self.list(&format!(
            "trashed = false and '{folder}' in parents and mimeType = 'application/octet-stream'"
        ))
    }
    pub fn list_kind(&self, folder: &str, kind: ObjectKind) -> Result<Vec<File>> {
        id(folder)?;
        self.verify_folder(folder)?;
        self.list(&format!(
            "trashed = false and '{folder}' in parents and mimeType = '{}'",
            kind.mime()
        ))
    }
    fn list(&self, query: &str) -> Result<Vec<File>> {
        self.check_token()?;
        let mut files = vec![];
        let mut next = String::new();
        let mut seen = std::collections::BTreeSet::new();
        loop {
            let r = self
                .client
                .get(&self.files_url)
                .bearer_auth(self.token.value.as_str())
                .query(&[
                    ("q", query),
                    ("spaces", "drive"),
                    ("pageSize", "1000"),
                    (
                        "fields",
                        "nextPageToken,incompleteSearch,files(id,name,mimeType,parents,trashed,version,size)",
                    ),
                    ("pageToken", next.as_str()),
                    ("supportsAllDrives", "true"),
                    ("includeItemsFromAllDrives", "true"),
                ])
                .timeout(crate::resources::request_timeout())
            .send()
                .map_err(|_| "network_unavailable")?;
            let page: Page = serde_json::from_slice(&read(r, 1024 * 1024)?)
                .map_err(|_| "drive_invalid_response")?;
            if page.incomplete_search {
                return Err("drive_incomplete_listing".into());
            }
            files.extend(page.files);
            if files.len() > MAX_OBJECTS {
                return Err("drive_response_limit".into());
            }
            match page.next_page_token {
                Some(n) if !n.is_empty() => {
                    if !seen.insert(n.clone()) || seen.len() > MAX_OBJECTS / 100 + 1 {
                        return Err("drive_invalid_response".into());
                    }
                    next = n;
                }
                _ => break,
            }
        }
        Ok(files)
    }
    pub fn metadata(&self, file: &str) -> Result<File> {
        id(file)?;
        self.check_token()?;
        let r = self
            .client
            .get(format!("{}/{file}", self.files_url))
            .bearer_auth(self.token.value.as_str())
            .query(&[
                ("fields", "id,name,mimeType,parents,trashed"),
                ("supportsAllDrives", "true"),
            ])
            .timeout(crate::resources::request_timeout())
            .send()
            .map_err(|_| "network_unavailable")?;
        let info: File =
            serde_json::from_slice(&read(r, 65536)?).map_err(|_| "drive_invalid_response")?;
        if info.id != file || info.trashed {
            return Err("drive_not_found".into());
        }
        Ok(info)
    }
    pub fn verify_folder(&self, folder: &str) -> Result<File> {
        let f = self.metadata(folder)?;
        if f.mime_type != FOLDER {
            return Err("drive_not_folder".into());
        }
        Ok(f)
    }
    pub fn create_folder(&self, allocated: &str, name: &str) -> Result<File> {
        id(allocated)?;
        self.check_token()?;
        if name.trim().is_empty() || name.len() > 128 || name.chars().any(char::is_control) {
            return Err("invalid_folder_name".into());
        }
        let r = self
            .client
            .post(&self.files_url)
            .bearer_auth(self.token.value.as_str())
            .query(&[("fields", "id,name,mimeType,parents,trashed")])
            .header("Content-Type", "application/json")
            .body(super::traffic::upload(
                serde_json::to_vec(&json!({"id":allocated,"name":name,"mimeType":FOLDER}))
                    .map_err(|_| "drive_invalid_metadata")?,
            ))
            .timeout(crate::resources::request_timeout())
            .send()
            .map_err(|_| "network_unavailable")?;
        serde_json::from_slice(&read(r, 65536)?).map_err(|_| "drive_invalid_response".into())
    }
    /// Returns only after Drive acknowledges. An uncertain response requires reconciliation by ID.
    pub fn upload(
        &self,
        folder: &str,
        allocated: &str,
        key: &SpaceKey,
        bundle: &Bundle,
    ) -> Result<File> {
        self.upload_kind(folder, allocated, key, bundle, ObjectKind::Session)
    }
    pub fn upload_kind(
        &self,
        folder: &str,
        allocated: &str,
        key: &SpaceKey,
        bundle: &Bundle,
        kind: ObjectKind,
    ) -> Result<File> {
        id(allocated)?;
        self.verify_folder(folder)?;
        let metadata = json!({"id":allocated,"name":format!("{allocated}.bas"),"parents":[folder],"mimeType":kind.mime()});
        let boundary = format!("bastet_{}", uuid::Uuid::new_v4().simple());
        let encrypted = key.seal(bundle)?;
        let mut bytes=format!("--{boundary}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n{metadata}\r\n--{boundary}\r\nContent-Type: application/octet-stream\r\n\r\n").into_bytes();
        bytes.extend_from_slice(&encrypted);
        bytes.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
        let r = self
            .client
            .post(&self.upload_url)
            .bearer_auth(self.token.value.as_str())
            .query(&[
                ("uploadType", "multipart"),
                ("supportsAllDrives", "true"),
                ("fields", "id,name,mimeType,parents,trashed"),
            ])
            .header(
                "Content-Type",
                format!("multipart/related; boundary={boundary}"),
            )
            .body(super::traffic::upload(bytes))
            .timeout(crate::resources::request_timeout())
            .send()
            .map_err(|_| "network_unavailable")?;
        serde_json::from_slice(&read(r, 65536)?).map_err(|_| "drive_invalid_response".into())
    }
    pub fn download(
        &self,
        folder: &str,
        file: &str,
        space: &str,
        key: &SpaceKey,
    ) -> Result<Bundle> {
        self.download_kind(folder, file, space, key, ObjectKind::Session)
    }
    pub fn download_kind(
        &self,
        folder: &str,
        file: &str,
        space: &str,
        key: &SpaceKey,
        kind: ObjectKind,
    ) -> Result<Bundle> {
        id(folder)?;
        let meta = self.metadata(file)?;
        if !meta.parents.iter().any(|v| v == folder) || meta.mime_type != kind.mime() {
            return Err("drive_wrong_parent_or_type".into());
        }
        let r = self
            .client
            .get(format!("{}/{file}", self.files_url))
            .bearer_auth(self.token.value.as_str())
            .query(&[("alt", "media"), ("supportsAllDrives", "true")])
            .timeout(crate::resources::request_timeout())
            .send()
            .map_err(|_| "network_unavailable")?;
        key.open(space, &read(r, MAX_ENCRYPTED)?)
    }
    /// Only the caller's own device report may be replaced; caller supplies its expected profile.
    pub fn update_device(
        &self,
        folder: &str,
        file: &str,
        space: &str,
        key: &SpaceKey,
        bundle: &Bundle,
    ) -> Result<()> {
        let previous = self.download_kind(folder, file, space, key, ObjectKind::Device)?;
        if previous.snapshot.stream != bundle.snapshot.stream {
            return Err("device_report_mismatch".into());
        }
        let r = self
            .client
            .patch(format!("{}/{file}", self.upload_url))
            .bearer_auth(self.token.value.as_str())
            .query(&[("uploadType", "media"), ("supportsAllDrives", "true")])
            .header("Content-Type", ObjectKind::Device.mime())
            .body(super::traffic::upload(key.seal(bundle)?))
            .timeout(crate::resources::request_timeout())
            .send()
            .map_err(|_| "network_unavailable")?;
        read(r, 65536)?;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_backoff_and_untrusted_identifiers() {
        assert_eq!(
            retry_delay(429, 2, None, 200),
            Some(Duration::from_millis(4200))
        );
        assert_eq!(
            retry_delay(503, 0, Some(99999), 0),
            Some(Duration::from_secs(120))
        );
        assert!(retry_delay(403, 0, None, 0).is_none());
        assert!(retry_delay(429, 5, None, 0).is_none());
        for value in [
            "../secret",
            "a' or trashed=false",
            "https://other",
            "",
            "a/b",
        ] {
            assert!(id(value).is_err());
        }
    }
}

#[cfg(test)]
mod http_tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
    };
    // Test-only HTTP origin; production constructors always use fixed HTTPS Google endpoints.
    fn fixture(responses: Vec<(u16, Vec<u8>)>) -> (Drive, thread::JoinHandle<Vec<Vec<u8>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let worker = thread::spawn(move || {
            let mut requests = vec![];
            for (status, body) in responses {
                let start = std::time::Instant::now();
                let mut stream = loop {
                    match listener.accept() {
                        Ok((s, _)) => break s,
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            assert!(start.elapsed() < Duration::from_secs(10));
                            thread::sleep(Duration::from_millis(10));
                        }
                        Err(e) => panic!("{e}"),
                    }
                };
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut bytes = vec![];
                let mut chunk = [0u8; 4096];
                loop {
                    let n = stream.read(&mut chunk).unwrap();
                    assert!(n > 0);
                    bytes.extend_from_slice(&chunk[..n]);
                    if let Some(end) = bytes.windows(4).position(|v| v == b"\r\n\r\n") {
                        let head = String::from_utf8_lossy(&bytes[..end]).to_lowercase();
                        let len = head
                            .lines()
                            .find_map(|l| {
                                l.strip_prefix("content-length:")
                                    .and_then(|v| v.trim().parse::<usize>().ok())
                            })
                            .unwrap_or(0);
                        if bytes.len() >= end + 4 + len {
                            break;
                        }
                    }
                }
                requests.push(bytes);
                write!(stream,"HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n",body.len()).unwrap();
                stream.write_all(&body).unwrap();
            }
            requests
        });
        (
            Drive {
                client: Client::builder()
                    .no_proxy()
                    .timeout(Duration::from_secs(5))
                    .redirect(reqwest::redirect::Policy::none())
                    .build()
                    .unwrap(),
                token: super::super::oauth::fixture_token(),
                files_url: format!("{base}/files"),
                upload_url: format!("{base}/upload"),
                about_url: format!("{base}/about"),
            },
            worker,
        )
    }
    #[test]
    fn account_uses_bounded_about_user_and_rejects_missing_identity() {
        let (drive, server) = fixture(vec![(200, br#"{"user":{"permissionId":"123","displayName":"Cat","emailAddress":"cat@example.test"}}"#.to_vec())]);
        assert_eq!(drive.account().unwrap().permission_id, "123");
        let requests = server.join().unwrap();
        let request = String::from_utf8_lossy(&requests[0]);
        assert!(request.starts_with("GET /about?fields="));
        assert!(request.contains("permissionId"));
        for body in [
            br#"{"user":{"displayName":"Cat"}}"#.as_slice(),
            br#"{"user":{"permissionId":"","displayName":null}}"#,
        ] {
            let (drive, server) = fixture(vec![(200, body.to_vec())]);
            assert_eq!(drive.account().unwrap_err(), "drive_invalid_account");
            server.join().unwrap();
        }
    }
    fn folder() -> Vec<u8> {
        json!({"id":"folder","name":"Bastet","mimeType":FOLDER})
            .to_string()
            .into_bytes()
    }
    fn bundle() -> Bundle {
        use crate::sync::bundle::{Entry, Snapshot, Stream};
        Bundle::new(Snapshot {
            schema: 1,
            space: "space".into(),
            device: "a".into(),
            stream: Stream {
                agent: "codex".into(),
                profile: "p".into(),
                conversation: "c".into(),
            },
            parents: vec![],
            files: std::collections::BTreeMap::from([(
                "sample.txt".into(),
                Entry::new("PRIVATE FIXTURE".into()),
            )]),
        })
        .unwrap()
    }
    #[test]
    fn encrypted_http_upload_and_download() {
        let key = SpaceKey::generate().unwrap();
        let bundle = bundle();
        let wire = key.seal(&bundle).unwrap();
        let meta=json!({"id":"object","name":"object.bas","mimeType":"application/octet-stream","parents":["folder"]}).to_string().into_bytes();
        let (drive, server) = fixture(vec![
            (200, folder()),
            (200, meta.clone()),
            (200, meta),
            (200, wire),
        ]);
        assert_eq!(
            drive.upload("folder", "object", &key, &bundle).unwrap().id,
            "object"
        );
        assert_eq!(
            drive.download("folder", "object", "space", &key).unwrap(),
            bundle
        );
        let requests = server.join().unwrap();
        let upload = String::from_utf8_lossy(&requests[1]);
        assert!(upload.starts_with("POST /upload?"));
        assert!(upload.contains("multipart/related"));
        assert!(upload.contains("\"id\":\"object\""));
        assert!(!upload.contains("PRIVATE FIXTURE"));
    }
    #[test]
    fn pagination_and_errors_are_not_reported_as_empty_success() {
        let (drive, server) = fixture(vec![
            (200, br#"{"files":[],"nextPageToken":"next"}"#.to_vec()),
            (200, br#"{"files":[]}"#.to_vec()),
        ]);
        assert!(drive.list_folders().unwrap().is_empty());
        let requests = server.join().unwrap();
        assert!(String::from_utf8_lossy(&requests[1]).contains("pageToken=next"));
        for (status, expected) in [
            (401, "reauth_required"),
            (403, "drive_forbidden"),
            (429, "drive_rate_limited"),
            (503, "drive_temporary"),
        ] {
            let (drive, server) = fixture(vec![(status, b"SECRET PROVIDER BODY".to_vec())]);
            assert_eq!(drive.list_folders().unwrap_err(), expected);
            server.join().unwrap();
        }
        let (drive, server) = fixture(vec![(
            200,
            br#"{"incompleteSearch":true,"files":[]}"#.to_vec(),
        )]);
        assert_eq!(
            drive.list_folders().unwrap_err(),
            "drive_incomplete_listing"
        );
        server.join().unwrap();
    }
    #[test]
    fn duplicate_id_and_wrong_parent_never_overwrite_or_download() {
        let (drive, server) = fixture(vec![(409, vec![])]);
        assert_eq!(
            drive.create_folder("allocated", "Bastet").unwrap_err(),
            "drive_id_exists"
        );
        assert_eq!(server.join().unwrap().len(), 1);
        let meta=json!({"id":"object","name":"object.bas","mimeType":"application/octet-stream","parents":["other"]}).to_string().into_bytes();
        let (drive, server) = fixture(vec![(200, meta)]);
        assert_eq!(
            drive
                .download("folder", "object", "space", &SpaceKey::generate().unwrap())
                .unwrap_err(),
            "drive_wrong_parent_or_type"
        );
        assert_eq!(server.join().unwrap().len(), 1);
    }
    #[test]
    fn typed_objects_are_separate_and_wrong_type_cannot_be_read() {
        for kind in [
            ObjectKind::Session,
            ObjectKind::Portable,
            ObjectKind::Device,
        ] {
            let (drive, server) =
                fixture(vec![(200, folder()), (200, br#"{"files":[]}"#.to_vec())]);
            assert!(drive.list_kind("folder", kind).unwrap().is_empty());
            let requests = server.join().unwrap();
            let decoded = url::form_urlencoded::parse(
                String::from_utf8_lossy(&requests[1])
                    .split(" HTTP/")
                    .next()
                    .unwrap()
                    .trim_start_matches("GET /files?")
                    .as_bytes(),
            )
            .into_owned()
            .collect::<std::collections::BTreeMap<_, _>>();
            assert!(decoded["q"].contains(kind.mime()));
            assert!(decoded["fields"].contains("size"));
        }
        let meta =
            json!({"id":"object","name":"object.bas","mimeType":ObjectKind::Portable.mime(),"parents":["folder"]})
                .to_string()
                .into_bytes();
        let (drive, server) = fixture(vec![(200, meta)]);
        assert_eq!(
            drive
                .download("folder", "object", "space", &SpaceKey::generate().unwrap())
                .unwrap_err(),
            "drive_wrong_parent_or_type"
        );
        assert_eq!(server.join().unwrap().len(), 1);
    }
    #[test]
    fn device_update_reuses_id_but_refuses_another_device_stream() {
        let key = SpaceKey::generate().unwrap();
        let original = bundle();
        let meta = json!({"id":"object","name":"object.bas","mimeType":ObjectKind::Device.mime(),"parents":["folder"]})
            .to_string()
            .into_bytes();
        let (drive, server) = fixture(vec![
            (200, meta.clone()),
            (200, key.seal(&original).unwrap()),
            (200, b"{}".to_vec()),
        ]);
        drive
            .update_device("folder", "object", "space", &key, &original)
            .unwrap();
        let requests = server.join().unwrap();
        assert!(String::from_utf8_lossy(&requests[2]).starts_with("PATCH /upload/object?"));
        assert!(!String::from_utf8_lossy(&requests[2]).contains("PRIVATE FIXTURE"));
        let mut other = original.snapshot.clone();
        other.stream.profile = "other".into();
        let (drive, server) = fixture(vec![(200, meta), (200, key.seal(&original).unwrap())]);
        assert_eq!(
            drive
                .update_device(
                    "folder",
                    "object",
                    "space",
                    &key,
                    &Bundle::new(other).unwrap()
                )
                .unwrap_err(),
            "device_report_mismatch"
        );
        assert_eq!(server.join().unwrap().len(), 2);
    }
}
