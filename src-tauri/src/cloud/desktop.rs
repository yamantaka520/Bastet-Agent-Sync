//! Shared connection state and an isolated native cryptographic diagnostic.
use super::{crypto::SpaceKey, drive::Drive, Result};
use serde::Serialize;
use std::sync::{Arc, Mutex};
#[derive(Default, Clone)]
pub struct CloudState(pub Arc<Mutex<Option<Drive>>>, pub BrowserWaits);

#[derive(Default, Clone)]
pub struct BrowserWaits(Arc<Mutex<Option<Arc<std::sync::atomic::AtomicBool>>>>);
pub struct BrowserWait {
    owner: BrowserWaits,
    pub cancelled: Arc<std::sync::atomic::AtomicBool>,
}
impl BrowserWaits {
    pub fn begin(&self) -> Result<BrowserWait> {
        let mut slot = self.0.lock().map_err(|_| "cloud_busy")?;
        if slot.is_some() {
            return Err("cloud_busy".into());
        }
        let cancelled = Arc::new(std::sync::atomic::AtomicBool::new(false));
        *slot = Some(cancelled.clone());
        Ok(BrowserWait {
            owner: self.clone(),
            cancelled,
        })
    }
    pub fn cancel(&self) -> Result<bool> {
        let slot = self.0.lock().map_err(|_| "cloud_busy")?;
        if let Some(flag) = slot.as_ref() {
            flag.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
impl BrowserWait {
    pub fn complete<T>(self, result: Result<T>) -> Result<T> {
        let mut slot = self.owner.0.lock().map_err(|_| "cloud_busy")?;
        *slot = None;
        if self.cancelled.load(std::sync::atomic::Ordering::SeqCst) {
            Err("oauth_cancelled".into())
        } else {
            result
        }
    }
}
impl Drop for BrowserWait {
    fn drop(&mut self) {
        if let Ok(mut slot) = self.owner.0.lock() {
            if slot
                .as_ref()
                .is_some_and(|flag| Arc::ptr_eq(flag, &self.cancelled))
            {
                *slot = None;
            }
        }
    }
}
#[tauri::command]
pub fn wizard_cancel_login(state: tauri::State<'_, CloudState>) -> Result<bool> {
    state.1.cancel()
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CryptoDiagnostic {
    verified: bool,
    recovery_verified: bool,
    tamper_rejected: bool,
}
#[tauri::command]
pub async fn run_crypto_diagnostic() -> Result<CryptoDiagnostic> {
    tauri::async_runtime::spawn_blocking(|| {
        use crate::sync::bundle::{Bundle, Entry, Snapshot, Stream};
        let bundle = Bundle::new(Snapshot {
            schema: 1,
            space: "isolated-crypto-check".into(),
            device: "synthetic".into(),
            stream: Stream {
                agent: "codex".into(),
                profile: "fixture".into(),
                conversation: "fixture".into(),
            },
            parents: vec![],
            files: std::collections::BTreeMap::from([(
                "sample.txt".into(),
                Entry::new("Synthetic encryption check / 三花貓".into()),
            )]),
        })?;
        let key = SpaceKey::generate()?;
        let bytes = key.seal(&bundle)?;
        let recovered = SpaceKey::recover(&key.recovery_code())?;
        let recovery_verified = recovered.open(&bundle.snapshot.space, &bytes)? == bundle;
        let mut changed: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|_| "crypto_check_failed")?;
        let encoded = changed["ciphertext"]
            .as_str()
            .ok_or("crypto_check_failed")?;
        let mut altered = encoded.to_owned();
        altered.replace_range(..1, if encoded.starts_with('A') { "B" } else { "A" });
        changed["ciphertext"] = serde_json::Value::String(altered);
        let tamper_rejected = recovered
            .open(
                &bundle.snapshot.space,
                &serde_json::to_vec(&changed).map_err(|_| "crypto_check_failed")?,
            )
            .is_err();
        if !recovery_verified || !tamper_rejected {
            return Err("crypto_check_failed".into());
        }
        Ok(CryptoDiagnostic {
            verified: true,
            recovery_verified,
            tamper_rejected,
        })
    })
    .await
    .map_err(|_| "crypto_check_failed".to_string())?
}

#[cfg(test)]
mod cancellation_tests {
    use super::*;
    #[test]
    fn cancel_is_scoped_to_browser_wait_and_retry_gets_fresh_flag() {
        let waits = BrowserWaits::default();
        assert!(!waits.cancel().unwrap());
        let pending = waits.begin().unwrap();
        assert!(waits.begin().is_err());
        assert!(waits.cancel().unwrap());
        assert_eq!(pending.complete(Ok("code")).unwrap_err(), "oauth_cancelled");
        assert!(!waits.cancel().unwrap());
        let next = waits.begin().unwrap();
        assert_eq!(next.complete(Ok("new-code")).unwrap(), "new-code");
        assert!(!waits.cancel().unwrap());
        drop(waits.begin().unwrap());
        assert!(!waits.cancel().unwrap());
    }
}
