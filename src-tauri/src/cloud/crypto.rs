use super::Result;
use crate::sync::bundle::{token, Bundle, MAX_WIRE};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD as B64, Engine};
use chacha20poly1305::{
    aead::{Aead, Payload},
    KeyInit, XChaCha20Poly1305, XNonce,
};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

const MAGIC: &[u8] = b"BastetAgentSync/encrypted/v1";
pub const MAX_ENCRYPTED: usize = MAX_WIRE as usize * 2;
// Never Serialize or Debug key material. A recovery code is an explicitly exported secret.
pub struct SpaceKey(Zeroizing<[u8; 32]>);
impl SpaceKey {
    pub fn generate() -> Result<Self> {
        let mut key = Zeroizing::new([0u8; 32]);
        OsRng
            .try_fill_bytes(key.as_mut())
            .map_err(|_| "random_unavailable")?;
        Ok(Self(key))
    }
    pub fn recovery_code(&self) -> Zeroizing<String> {
        Zeroizing::new(format!("bas1_{}", B64.encode(self.0.as_ref())))
    }
    pub fn recover(code: &str) -> Result<Self> {
        if code.len() != 48 || !code.starts_with("bas1_") {
            return Err("invalid_recovery_key".into());
        }
        let decoded = Zeroizing::new(B64.decode(&code[5..]).map_err(|_| "invalid_recovery_key")?);
        let mut key = Zeroizing::new([0u8; 32]);
        if decoded.len() != 32 {
            return Err("invalid_recovery_key".into());
        }
        key.copy_from_slice(&decoded);
        Ok(Self(key))
    }
    pub fn seal(&self, bundle: &Bundle) -> Result<Vec<u8>> {
        bundle.validate()?;
        let plain = Zeroizing::new(serde_json::to_vec(bundle).map_err(|_| "invalid_bundle")?);
        if plain.len() > MAX_WIRE as usize {
            return Err("bundle_limit".into());
        }
        let mut nonce = [0u8; 24];
        OsRng
            .try_fill_bytes(&mut nonce)
            .map_err(|_| "random_unavailable")?;
        let aad = aad(&bundle.snapshot.space)?;
        let cipher =
            XChaCha20Poly1305::new_from_slice(self.0.as_ref()).map_err(|_| "encryption_failed")?;
        let ciphertext = cipher
            .encrypt(
                XNonce::from_slice(&nonce),
                Payload {
                    msg: &plain,
                    aad: &aad,
                },
            )
            .map_err(|_| "encryption_failed")?;
        serde_json::to_vec(&Envelope {
            version: 1,
            space: bundle.snapshot.space.clone(),
            nonce: B64.encode(nonce),
            ciphertext: B64.encode(ciphertext),
        })
        .map_err(|_| "encryption_failed".into())
    }
    pub fn open(&self, expected_space: &str, bytes: &[u8]) -> Result<Bundle> {
        if bytes.len() > MAX_ENCRYPTED {
            return Err("bundle_limit".into());
        }
        let envelope: Envelope = serde_json::from_slice(bytes).map_err(|_| "invalid_envelope")?;
        if envelope.version != 1 || envelope.space != expected_space {
            return Err("wrong_space_or_version".into());
        }
        let nonce = B64
            .decode(&envelope.nonce)
            .map_err(|_| "invalid_envelope")?;
        if nonce.len() != 24 {
            return Err("invalid_envelope".into());
        }
        let ciphertext = B64
            .decode(&envelope.ciphertext)
            .map_err(|_| "invalid_envelope")?;
        if ciphertext.len() > MAX_WIRE as usize + 16 {
            return Err("bundle_limit".into());
        }
        let cipher =
            XChaCha20Poly1305::new_from_slice(self.0.as_ref()).map_err(|_| "encryption_failed")?;
        let plaintext = Zeroizing::new(
            cipher
                .decrypt(
                    XNonce::from_slice(&nonce),
                    Payload {
                        msg: &ciphertext,
                        aad: &aad(expected_space)?,
                    },
                )
                .map_err(|_| "decrypt_failed")?,
        );
        let bundle: Bundle = serde_json::from_slice(&plaintext).map_err(|_| "invalid_bundle")?;
        bundle.validate()?;
        if bundle.snapshot.space != expected_space {
            return Err("wrong_space_or_version".into());
        }
        Ok(bundle)
    }
}
fn aad(space: &str) -> Result<Vec<u8>> {
    if !token(space) {
        return Err("invalid_space".into());
    }
    let mut result = MAGIC.to_vec();
    result.push(0);
    result.extend_from_slice(space.as_bytes());
    Ok(result)
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    version: u32,
    space: String,
    nonce: String,
    ciphertext: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::bundle::{Entry, Snapshot, Stream};
    use std::collections::BTreeMap;
    pub fn sample() -> Bundle {
        Bundle::new(Snapshot {
            schema: 1,
            space: "test-space".into(),
            device: "a".into(),
            stream: Stream {
                agent: "codex".into(),
                profile: "fixture".into(),
                conversation: "sample".into(),
            },
            parents: vec![],
            files: BTreeMap::from([(
                "session.txt".into(),
                Entry::new("繁體中文 / 日本語 / 한국어 / secret fixture".into()),
            )]),
        })
        .unwrap()
    }
    #[test]
    fn recovery_and_randomized_encryption_roundtrip() {
        let key = SpaceKey::generate().unwrap();
        let b = sample();
        let one = key.seal(&b).unwrap();
        let two = key.seal(&b).unwrap();
        assert_ne!(one, two);
        assert!(!String::from_utf8_lossy(&one).contains("secret fixture"));
        let recovered = SpaceKey::recover(&key.recovery_code()).unwrap();
        assert_eq!(recovered.open("test-space", &one).unwrap(), b);
    }
    #[test]
    fn tampering_wrong_key_space_and_version_fail_closed() {
        let key = SpaceKey::generate().unwrap();
        let bytes = key.seal(&sample()).unwrap();
        assert!(SpaceKey::generate()
            .unwrap()
            .open("test-space", &bytes)
            .is_err());
        assert!(key.open("other-space", &bytes).is_err());
        for field in ["nonce", "ciphertext"] {
            let mut e: Envelope = serde_json::from_slice(&bytes).unwrap();
            let s = if field == "nonce" {
                &mut e.nonce
            } else {
                &mut e.ciphertext
            };
            let mut decoded = B64.decode(&*s).unwrap();
            decoded[0] ^= 1;
            *s = B64.encode(decoded);
            assert!(key
                .open("test-space", &serde_json::to_vec(&e).unwrap())
                .is_err());
        }
        let mut e: Envelope = serde_json::from_slice(&bytes).unwrap();
        e.version = 2;
        assert!(key
            .open("test-space", &serde_json::to_vec(&e).unwrap())
            .is_err());
        assert!(SpaceKey::recover("password").is_err());
        assert!(key.open("test-space", &bytes[..bytes.len() - 1]).is_err());
    }
}
