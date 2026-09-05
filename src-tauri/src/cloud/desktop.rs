//! Shared connection state and an isolated native cryptographic diagnostic.
use super::{crypto::SpaceKey, drive::Drive, Result};
use serde::Serialize;
use std::sync::{Arc, Mutex};
#[derive(Default, Clone)]
pub struct CloudState(pub Arc<Mutex<Option<Drive>>>);
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
