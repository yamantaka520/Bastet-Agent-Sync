//! Explicit native credential stores; unavailable/locked stores never fall back to a file.
use super::{crypto::SpaceKey, Result};
use crate::sync::bundle::token;
use std::{
    collections::BTreeMap,
    sync::{LazyLock, Mutex},
};
use zeroize::Zeroizing;

pub trait SecretStore {
    fn read(&self, account: &str) -> Result<Option<Zeroizing<String>>>;
    fn write(&self, account: &str, value: &str) -> Result<()>;
    fn remove(&self, account: &str) -> Result<()>;
}
// All native callers share one process-local cache. Never persist it or serialize it.
pub struct NativeStore;
struct NativeBackend;
struct CachedStore<S> {
    backend: S,
    values: Mutex<BTreeMap<String, Zeroizing<String>>>,
}
impl<S: SecretStore> CachedStore<S> {
    fn new(backend: S) -> Self {
        Self {
            backend,
            values: Mutex::new(BTreeMap::new()),
        }
    }
    fn clear(&self) -> Result<()> {
        self.values
            .lock()
            .map_err(|_| "credential_store_unavailable")?
            .clear();
        Ok(())
    }
}
impl<S: SecretStore> SecretStore for CachedStore<S> {
    fn read(&self, account: &str) -> Result<Option<Zeroizing<String>>> {
        // Serialize misses and mutations, including the native access, so concurrent
        // callers cannot prompt twice or repopulate a deleted credential.
        let mut values = self
            .values
            .lock()
            .map_err(|_| "credential_store_unavailable")?;
        if let Some(value) = values.get(account) {
            return Ok(Some(value.clone()));
        }
        let value = self.backend.read(account)?;
        if let Some(value) = &value {
            values.insert(account.into(), value.clone());
        }
        Ok(value) // Missing entries and failures are never cached.
    }
    fn write(&self, account: &str, value: &str) -> Result<()> {
        let mut values = self
            .values
            .lock()
            .map_err(|_| "credential_store_unavailable")?;
        values.remove(account);
        self.backend.write(account, value)?;
        values.insert(account.into(), Zeroizing::new(value.into()));
        Ok(())
    }
    fn remove(&self, account: &str) -> Result<()> {
        let mut values = self
            .values
            .lock()
            .map_err(|_| "credential_store_unavailable")?;
        values.remove(account);
        self.backend.remove(account)
    }
}
static CACHE: LazyLock<CachedStore<NativeBackend>> =
    LazyLock::new(|| CachedStore::new(NativeBackend));
impl NativeStore {
    pub fn clear_cache() -> Result<()> {
        CACHE.clear()
    }
}
impl SecretStore for NativeStore {
    fn read(&self, account: &str) -> Result<Option<Zeroizing<String>>> {
        CACHE.read(account)
    }
    fn write(&self, account: &str, value: &str) -> Result<()> {
        CACHE.write(account, value)
    }
    fn remove(&self, account: &str) -> Result<()> {
        CACHE.remove(account)
    }
}

impl NativeStore {
    fn entry(account: &str) -> Result<keyring::Entry> {
        if account.is_empty() || account.len() > 256 {
            return Err("invalid_credential_id".into());
        }
        keyring::Entry::new("tw.bastet.agent-sync", account)
            .map_err(|_| "credential_store_unavailable".into())
    }
}
impl SecretStore for NativeBackend {
    fn read(&self, account: &str) -> Result<Option<Zeroizing<String>>> {
        match NativeStore::entry(account)?.get_password() {
            Ok(v) => Ok(Some(Zeroizing::new(v))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err("credential_store_unavailable".into()),
        }
    }
    fn write(&self, account: &str, value: &str) -> Result<()> {
        NativeStore::entry(account)?
            .set_password(value)
            .map_err(|_| "credential_store_unavailable".into())
    }
    fn remove(&self, account: &str) -> Result<()> {
        match NativeStore::entry(account)?.delete_credential() {
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

#[cfg(test)]
mod cache_tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    };
    #[derive(Default)]
    struct Backend {
        values: Mutex<BTreeMap<String, String>>,
        reads: AtomicUsize,
        fail: AtomicBool,
    }
    impl SecretStore for Arc<Backend> {
        fn read(&self, account: &str) -> Result<Option<Zeroizing<String>>> {
            self.reads.fetch_add(1, Ordering::SeqCst);
            if self.fail.load(Ordering::SeqCst) {
                return Err("locked".into());
            }
            Ok(self
                .values
                .lock()
                .unwrap()
                .get(account)
                .cloned()
                .map(Zeroizing::new))
        }
        fn write(&self, account: &str, value: &str) -> Result<()> {
            if self.fail.load(Ordering::SeqCst) {
                return Err("locked".into());
            }
            self.values
                .lock()
                .unwrap()
                .insert(account.into(), value.into());
            Ok(())
        }
        fn remove(&self, account: &str) -> Result<()> {
            if self.fail.load(Ordering::SeqCst) {
                return Err("locked".into());
            }
            self.values.lock().unwrap().remove(account);
            Ok(())
        }
    }
    #[test]
    fn concurrent_reads_access_backend_once_and_clear_reloads() {
        let backend = Arc::new(Backend::default());
        backend.write("space:a", "old").unwrap();
        let cache = Arc::new(CachedStore::new(backend.clone()));
        let threads: Vec<_> = (0..12)
            .map(|_| {
                let cache = cache.clone();
                std::thread::spawn(move || {
                    assert_eq!(&**cache.read("space:a").unwrap().as_ref().unwrap(), "old")
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
        assert_eq!(backend.reads.load(Ordering::SeqCst), 1);
        backend.write("space:a", "external").unwrap();
        cache.clear().unwrap();
        assert_eq!(&*cache.read("space:a").unwrap().unwrap(), "external");
        assert_eq!(backend.reads.load(Ordering::SeqCst), 2);
    }
    #[test]
    fn missing_and_failed_reads_can_retry_and_accounts_are_isolated() {
        let backend = Arc::new(Backend::default());
        let cache = CachedStore::new(backend.clone());
        assert!(cache.read("a").unwrap().is_none());
        backend.write("a", "A").unwrap();
        backend.fail.store(true, Ordering::SeqCst);
        assert!(cache.read("a").is_err());
        backend.fail.store(false, Ordering::SeqCst);
        assert_eq!(&*cache.read("a").unwrap().unwrap(), "A");
        cache.write("b", "B").unwrap();
        assert_eq!(&*cache.read("b").unwrap().unwrap(), "B");
        assert_eq!(&*cache.read("a").unwrap().unwrap(), "A");
    }
    #[test]
    fn writes_rotate_and_failed_mutations_evict_cached_values() {
        let backend = Arc::new(Backend::default());
        let cache = CachedStore::new(backend.clone());
        cache.write("a", "old").unwrap();
        cache.write("a", "new").unwrap();
        assert_eq!(&*cache.read("a").unwrap().unwrap(), "new");
        backend.fail.store(true, Ordering::SeqCst);
        assert!(cache.write("a", "unsaved").is_err());
        assert!(cache.read("a").is_err());
        backend.fail.store(false, Ordering::SeqCst);
        assert_eq!(&*cache.read("a").unwrap().unwrap(), "new");
        backend.fail.store(true, Ordering::SeqCst);
        assert!(cache.remove("a").is_err());
        assert!(cache.read("a").is_err());
        backend.fail.store(false, Ordering::SeqCst);
        cache.remove("a").unwrap();
        assert!(cache.read("a").unwrap().is_none());
    }
}
