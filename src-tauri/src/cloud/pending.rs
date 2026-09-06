//! Durable, single-operation folder creation journal. Preserve IDs across uncertain responses.
use super::{
    drive::{Drive, File},
    Result,
};
use crate::sync::storage;
use serde::{Deserialize, Serialize};
use std::path::Path;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Pending {
    client: String,
    id: String,
    name: String,
    completed: Option<File>,
}
pub trait FolderApi {
    fn allocate(&self) -> Result<String>;
    fn create(&self, id: &str, name: &str) -> Result<File>;
    fn inspect(&self, id: &str) -> Result<File>;
}
impl FolderApi for Drive {
    fn allocate(&self) -> Result<String> {
        self.allocate_id()
    }
    fn create(&self, id: &str, name: &str) -> Result<File> {
        self.create_folder(id, name)
    }
    fn inspect(&self, id: &str) -> Result<File> {
        self.metadata(id)
    }
}
pub fn create(root: &Path, client: &str, name: &str, api: &impl FolderApi) -> Result<File> {
    if name.trim().is_empty() || name.len() > 128 || name.chars().any(char::is_control) {
        return Err("invalid_folder_name".into());
    }
    storage::directory(root)?;
    let _lock = storage::lock(&root.join("folder-create.lock"))?;
    let path = root.join("folder-create.json");
    let previous = match std::fs::symlink_metadata(&path) {
        Ok(_) => Some(
            serde_json::from_slice::<Pending>(&storage::read(&path, 65536)?)
                .map_err(|_| "cloud_journal_invalid")?,
        ),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(_) => return Err("store_unavailable".into()),
    };
    let mut pending = match previous {
        Some(p) if p.client == client && p.name == name => p,
        Some(p) if p.completed.is_none() => return Err("cloud_creation_pending".into()),
        _ => Pending {
            client: client.into(),
            id: api.allocate()?,
            name: name.into(),
            completed: None,
        },
    };
    if let Some(file) = pending.completed {
        return Ok(file);
    }
    storage::replace(
        &path,
        &serde_json::to_vec(&pending).map_err(|_| "cloud_journal_invalid")?,
    )?;
    let file = match api.create(&pending.id, &pending.name) {
        Ok(f) => f,
        Err(e) if e == "drive_id_exists" => api.inspect(&pending.id)?,
        Err(e) => return Err(e),
    };
    if file.id != pending.id
        || file.name != pending.name
        || file.mime_type != "application/vnd.google-apps.folder"
        || file.trashed
    {
        return Err("drive_invalid_response".into());
    }
    pending.completed = Some(file.clone());
    storage::replace(
        &path,
        &serde_json::to_vec(&pending).map_err(|_| "cloud_journal_invalid")?,
    )?;
    Ok(file)
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    struct Api {
        calls: Cell<u32>,
    }
    impl FolderApi for Api {
        fn allocate(&self) -> Result<String> {
            Ok("stable-id".into())
        }
        fn create(&self, id: &str, _: &str) -> Result<File> {
            assert_eq!(id, "stable-id");
            let n = self.calls.get();
            self.calls.set(n + 1);
            Err(if n == 0 {
                "network_unavailable"
            } else {
                "drive_id_exists"
            }
            .into())
        }
        fn inspect(&self, id: &str) -> Result<File> {
            Ok(File {
                size: None,
                id: id.into(),
                name: "Bastet".into(),
                mime_type: "application/vnd.google-apps.folder".into(),
                parents: vec![],
                trashed: false,
                version: None,
            })
        }
    }
    #[test]
    fn uncertain_creation_recovers_same_id_after_journal_reopen() {
        let temp = tempfile::tempdir().unwrap();
        let api = Api {
            calls: Cell::new(0),
        };
        assert_eq!(
            create(temp.path(), "client", "Bastet", &api).unwrap_err(),
            "network_unavailable"
        );
        assert_eq!(
            create(temp.path(), "client", "Other", &api).unwrap_err(),
            "cloud_creation_pending"
        );
        assert_eq!(
            create(temp.path(), "client", "Bastet", &api).unwrap().id,
            "stable-id"
        );
        create(temp.path(), "client", "Bastet", &api).unwrap();
        assert_eq!(api.calls.get(), 2);
    }
}
