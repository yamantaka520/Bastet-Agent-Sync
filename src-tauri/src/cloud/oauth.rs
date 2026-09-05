use super::{vault::SecretStore, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD as B64, Engine};
use rand::{rngs::OsRng, RngCore};
use reqwest::blocking::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
    time::{Duration, Instant},
};
use url::Url;
use zeroize::Zeroizing;

pub const SCOPE: &str = "https://www.googleapis.com/auth/drive.file";
const AUTH: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN: &str = "https://oauth2.googleapis.com/token";
// Desktop-client metadata supplied by the distributor; not an application authentication secret.
pub struct ClientConfig {
    pub id: String,
    pub secret: Option<Zeroizing<String>>,
}
impl ClientConfig {
    pub fn validate(&self) -> Result<()> {
        if self.id.len() > 256
            || !self.id.ends_with(".apps.googleusercontent.com")
            || !self
                .id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b".-_".contains(&c))
        {
            return Err("oauth_not_configured".into());
        }
        Ok(())
    }
}
pub struct Authorization {
    listener: TcpListener,
    verifier: Zeroizing<String>,
    state: Zeroizing<String>,
    redirect: String,
    started: Instant,
    pub url: Url,
}
fn random_token() -> Result<Zeroizing<String>> {
    let mut bytes = [0u8; 32];
    OsRng
        .try_fill_bytes(&mut bytes)
        .map_err(|_| "random_unavailable")?;
    Ok(Zeroizing::new(B64.encode(bytes)))
}
impl Authorization {
    pub fn begin(config: &ClientConfig) -> Result<Self> {
        config.validate()?;
        let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(|_| "oauth_listener_failed")?;
        listener
            .set_nonblocking(true)
            .map_err(|_| "oauth_listener_failed")?;
        let redirect = format!(
            "http://127.0.0.1:{}/oauth/callback",
            listener
                .local_addr()
                .map_err(|_| "oauth_listener_failed")?
                .port()
        );
        let verifier = random_token()?;
        let state = random_token()?;
        let mut url = Url::parse(AUTH).map_err(|_| "oauth_not_configured")?;
        url.query_pairs_mut().extend_pairs([
            ("client_id", config.id.as_str()),
            ("redirect_uri", redirect.as_str()),
            ("response_type", "code"),
            ("scope", SCOPE),
            ("access_type", "offline"),
            ("prompt", "consent"),
            ("code_challenge_method", "S256"),
            (
                "code_challenge",
                B64.encode(Sha256::digest(verifier.as_bytes())).as_str(),
            ),
            ("state", state.as_str()),
        ]);
        Ok(Self {
            listener,
            verifier,
            state,
            redirect,
            started: Instant::now(),
            url,
        })
    }
    /// Called only after an explicit Connect action. Listener is short lived, never a daemon.
    pub fn finish(self, config: &ClientConfig, store: &impl SecretStore) -> Result<AccessToken> {
        loop {
            if self.started.elapsed() > Duration::from_secs(180) {
                return Err("oauth_timeout".into());
            }
            match self.listener.accept() {
                Ok((mut stream, peer)) => {
                    if !peer.ip().is_loopback() {
                        continue;
                    }
                    stream
                        .set_read_timeout(Some(Duration::from_secs(2)))
                        .map_err(|_| "oauth_listener_failed")?;
                    let mut bytes = Vec::new();
                    let mut chunk = [0u8; 1024];
                    while bytes.len() < 8192 && !bytes.windows(4).any(|x| x == b"\r\n\r\n") {
                        match stream.read(&mut chunk) {
                            Ok(0) | Err(_) => break,
                            Ok(n) => bytes.extend_from_slice(&chunk[..n]),
                        }
                    }
                    let result = parse_callback(&bytes, &self.state);
                    let ok = result.is_ok();
                    let response = if ok {
                        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nConnection: close\r\nCache-Control: no-store\r\n\r\nReturn to Bastet Agent Sync."
                    } else {
                        "HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    };
                    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
                    let _ = stream.write_all(response.as_bytes());
                    match result {
                        Ok(code) => {
                            return exchange(config, &self.redirect, &self.verifier, &code, store)
                        }
                        Err(e) if e == "oauth_denied" => return Err(e),
                        Err(_) => continue, // unrelated requests cannot consume the valid pending state
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(50))
                }
                Err(_) => return Err("oauth_listener_failed".into()),
            }
        }
    }
}
fn parse_callback(bytes: &[u8], state: &str) -> Result<Zeroizing<String>> {
    if bytes.len() > 8192 || !bytes.windows(4).any(|x| x == b"\r\n\r\n") {
        return Err("oauth_callback_invalid".into());
    }
    let text = std::str::from_utf8(bytes).map_err(|_| "oauth_callback_invalid")?;
    let parts: Vec<_> = text
        .lines()
        .next()
        .unwrap_or("")
        .split_whitespace()
        .collect();
    if parts.len() != 3
        || parts[0] != "GET"
        || parts[2] != "HTTP/1.1"
        || !parts[1].starts_with("/oauth/callback?")
    {
        return Err("oauth_callback_invalid".into());
    }
    let url = Url::parse(&format!("http://127.0.0.1{}", parts[1]))
        .map_err(|_| "oauth_callback_invalid")?;
    let pairs: Vec<_> = url.query_pairs().collect();
    let get = |key: &str| -> Result<String> {
        let found: Vec<_> = pairs.iter().filter(|(k, _)| k == key).collect();
        if found.len() != 1 {
            return Err("oauth_callback_invalid".into());
        }
        Ok(found[0].1.to_string())
    };
    if get("state")? != state {
        return Err("oauth_callback_invalid".into());
    }
    if pairs.iter().any(|(k, _)| k == "error") {
        return Err("oauth_denied".into());
    }
    let code = get("code")?;
    if code.is_empty() || code.len() > 2048 {
        return Err("oauth_callback_invalid".into());
    }
    Ok(Zeroizing::new(code))
}
pub struct AccessToken {
    pub(crate) value: Zeroizing<String>,
    expires: Instant,
}
impl AccessToken {
    pub fn expired(&self) -> bool {
        Instant::now() >= self.expires
    }
}
#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    token_type: String,
    expires_in: u64,
    refresh_token: Option<String>,
    scope: Option<String>,
}
impl Drop for TokenResponse {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        self.access_token.zeroize();
        self.refresh_token.zeroize();
    }
}
pub fn http() -> Result<Client> {
    Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|_| "network_unavailable".into())
}
fn request_token(form: &[(&str, &str)]) -> Result<TokenResponse> {
    let response = http()?
        .post(TOKEN)
        .form(form)
        .send()
        .map_err(|_| "network_unavailable")?;
    if !response.status().is_success() {
        return Err(if response.status().as_u16() == 400 {
            "reauth_required"
        } else {
            "oauth_exchange_failed"
        }
        .into());
    }
    let mut bytes = Zeroizing::new(Vec::new());
    response
        .take(65537)
        .read_to_end(&mut bytes)
        .map_err(|_| "oauth_exchange_failed")?;
    if bytes.len() > 65536 {
        return Err("oauth_exchange_failed".into());
    }
    let token: TokenResponse =
        serde_json::from_slice(&bytes).map_err(|_| "oauth_exchange_failed")?;
    if !token.token_type.eq_ignore_ascii_case("bearer")
        || token.access_token.is_empty()
        || token.expires_in < 60
        || token
            .scope
            .as_ref()
            .is_some_and(|s| !s.split_whitespace().any(|v| v == SCOPE))
    {
        return Err("oauth_scope_missing".into());
    }
    Ok(token)
}
fn access(token: &TokenResponse) -> AccessToken {
    AccessToken {
        value: Zeroizing::new(token.access_token.clone()),
        expires: Instant::now() + Duration::from_secs(token.expires_in.min(86400) - 30),
    }
}
fn account(config: &ClientConfig) -> String {
    format!("google:{}", crate::sync::bundle::hash(config.id.as_bytes()))
}
fn exchange(
    config: &ClientConfig,
    redirect: &str,
    verifier: &str,
    code: &str,
    store: &impl SecretStore,
) -> Result<AccessToken> {
    let mut form = vec![
        ("client_id", config.id.as_str()),
        ("redirect_uri", redirect),
        ("grant_type", "authorization_code"),
        ("code_verifier", verifier),
        ("code", code),
    ];
    if let Some(secret) = &config.secret {
        form.push(("client_secret", secret));
    }
    let token = request_token(&form)?;
    // A new account must never inherit an earlier account's refresh token.
    let refresh = token
        .refresh_token
        .as_ref()
        .filter(|v| !v.is_empty())
        .ok_or("oauth_refresh_missing")?;
    store.write(&account(config), refresh)?;
    Ok(access(&token))
}
pub fn reconnect(config: &ClientConfig, store: &impl SecretStore) -> Result<AccessToken> {
    config.validate()?;
    let refresh = store.read(&account(config))?.ok_or("reauth_required")?;
    let mut form = vec![
        ("client_id", config.id.as_str()),
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh.as_str()),
    ];
    if let Some(secret) = &config.secret {
        form.push(("client_secret", secret));
    }
    let token = request_token(&form)?;
    if let Some(new) = token.refresh_token.as_ref().filter(|v| !v.is_empty()) {
        store.write(&account(config), new)?;
    }
    Ok(access(&token))
}
pub fn forget_login(config: &ClientConfig, store: &impl SecretStore) -> Result<()> {
    config.validate()?;
    store.remove(&account(config))
}

#[cfg(test)]
pub(crate) fn fixture_token() -> AccessToken {
    AccessToken {
        value: Zeroizing::new("fixture-token".into()),
        expires: Instant::now() + Duration::from_secs(60),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn callback_requires_state_unique_code_and_exact_path() {
        let request =
            |q: &str| format!("GET /oauth/callback?{q} HTTP/1.1\r\nHost: localhost\r\n\r\n");
        assert_eq!(
            *parse_callback(request("state=abc&code=ok").as_bytes(), "abc").unwrap(),
            "ok"
        );
        for q in [
            "code=ok",
            "state=wrong&code=ok",
            "state=abc&code=a&code=b",
            "state=abc&state=abc&code=a",
            "state=abc&code=",
        ] {
            assert!(parse_callback(request(q).as_bytes(), "abc").is_err());
        }
        assert_eq!(
            parse_callback(request("state=abc&error=access_denied").as_bytes(), "abc").unwrap_err(),
            "oauth_denied"
        );
        assert!(parse_callback(b"GET /else?state=abc&code=ok HTTP/1.1\r\n\r\n", "abc").is_err());
    }
    #[test]
    fn pkce_s256_known_vector() {
        assert_eq!(
            B64.encode(Sha256::digest(
                b"dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"
            )),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
        assert!(ClientConfig {
            id: "bad".into(),
            secret: None
        }
        .validate()
        .is_err());
    }
}
