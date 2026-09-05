//! Explicit native credential stores; unavailable/locked stores never fall back to a file.
use super::{crypto::SpaceKey, Result};
use crate::sync::bundle::token;
use zeroize::Zeroizing;

pub trait SecretStore {
    fn read(&self, account: &str) -> Result<Option<Zeroizing<String>>>;
    fn write(&self, account: &str, value: &str) -> Result<()>;
    fn remove(&self, account: &str) -> Result<()>;
}
pub struct NativeStore;
impl NativeStore {
    fn entry(account: &str) -> Result<keyring::Entry> {
        if account.is_empty() || account.len() > 256 {
            return Err("invalid_credential_id".into());
        }
        keyring::Entry::new("tw.bastet.agent-sync", account)
            .map_err(|_| "credential_store_unavailable".into())
    }
}
impl SecretStore for NativeStore {
    fn read(&self, account: &str) -> Result<Option<Zeroizing<String>>> {
        match Self::entry(account)?.get_password() {
            Ok(v) => Ok(Some(Zeroizing::new(v))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err("credential_store_unavailable".into()),
        }
    }
    fn write(&self, account: &str, value: &str) -> Result<()> {
        Self::entry(account)?
            .set_password(value)
            .map_err(|_| "credential_store_unavailable".into())
    }
    fn remove(&self, account: &str) -> Result<()> {
        match Self::entry(account)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err("credential_store_unavailable".into()),
        }
    }
}
pub fn save_space_key(store: &impl SecretStore, space: &str, key: &SpaceKey) -> Result<()> {
    if !token(space) {
        return Err("invalid_space".into());
    }
    let account = format!("space:{space}");
    // Joining/recovering must not silently replace a key already in use.
    let code = key.recovery_code();
    if let Some(existing) = store.read(&account)? {
        if *existing != *code {
            return Err("space_key_exists".into());
        }
        return Ok(());
    }
    store.write(&account, &code)
}
pub fn load_space_key(store: &impl SecretStore, space: &str) -> Result<SpaceKey> {
    if !token(space) {
        return Err("invalid_space".into());
    }
    SpaceKey::recover(
        &store
            .read(&format!("space:{space}"))?
            .ok_or("space_key_missing")?,
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, collections::BTreeMap};
    #[derive(Default)]
    struct Memory(RefCell<BTreeMap<String, String>>);
    impl SecretStore for Memory {
        fn read(&self, a: &str) -> Result<Option<Zeroizing<String>>> {
            Ok(self.0.borrow().get(a).cloned().map(Zeroizing::new))
        }
        fn write(&self, a: &str, v: &str) -> Result<()> {
            self.0.borrow_mut().insert(a.into(), v.into());
            Ok(())
        }
        fn remove(&self, a: &str) -> Result<()> {
            self.0.borrow_mut().remove(a);
            Ok(())
        }
    }
    #[test]
    fn recovery_does_not_overwrite_existing_space_key() {
        let store = Memory::default();
        let key = SpaceKey::generate().unwrap();
        save_space_key(&store, "s", &key).unwrap();
        save_space_key(&store, "s", &key).unwrap();
        assert!(save_space_key(&store, "s", &SpaceKey::generate().unwrap()).is_err());
        assert_eq!(
            *load_space_key(&store, "s").unwrap().recovery_code(),
            *key.recovery_code()
        );
        assert!(load_space_key(&store, "missing").is_err());
    }
}
