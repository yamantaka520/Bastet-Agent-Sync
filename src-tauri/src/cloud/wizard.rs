//! Persistent setup state. Completion is recorded only after the corresponding operation succeeds.
use super::{
    crypto::SpaceKey,
    queue::{proof_bundle, Binding, Objects},
    vault::{load_space_key, save_space_key, SecretStore},
    Result,
};
use crate::sync::{bundle::token, storage};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    path::{Path, PathBuf},
};
use zeroize::Zeroizing;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Wizard {
    pub schema: u32,
    pub session: String,
    pub mode: String,
    pub page: usize,
    pub client_id: Option<String>,
    pub client_source: Option<String>,
    pub authorized: bool,
    #[serde(default)]
    pub account: Option<super::drive::Account>,
    pub folder_id: Option<String>,
    pub folder_name: Option<String>,
    pub binding: Option<Binding>,
    pub recovery_saved: bool,
    pub proof_verified: bool,
    pub complete: bool,
}
impl Default for Wizard {
    fn default() -> Self {
        Self {
            schema: 1,
            session: uuid::Uuid::new_v4().to_string(),
            mode: "guided".into(),
            page: 0,
            client_id: None,
            client_source: None,
            authorized: false,
            account: None,
            folder_id: None,
            folder_name: None,
            binding: None,
            recovery_saved: false,
            proof_verified: false,
            complete: false,
        }
    }
}
impl Wizard {
    pub fn next(&self) -> usize {
        if self.client_id.is_none() {
            0
        } else if !self.authorized {
            1
        } else if self.folder_id.is_none() {
            2
        } else if !self.proof_verified || !self.recovery_saved {
            3
        } else {
            4
        }
    }
    fn validate(&self) -> Result<()> {
        if self.schema != 1
            || !token(&self.session)
            || !["guided", "manual"].contains(&self.mode.as_str())
            || self.page > 4
        {
            return Err("wizard_corrupt".into());
        }
        if self.client_id.is_some() != self.client_source.is_some()
            || self
                .client_source
                .as_ref()
                .is_some_and(|s| !matches!(s.as_str(), "build" | "imported"))
        {
            return Err("wizard_corrupt".into());
        }
        if let Some(id) = &self.client_id {
            super::oauth::ClientConfig {
                id: id.clone(),
                secret: None,
            }
            .validate()
            .map_err(|_| "wizard_corrupt")?;
        }
        if let Some(account) = &self.account {
            account.validate().map_err(|_| "wizard_corrupt")?;
            if !self.authorized {
                return Err("wizard_corrupt".into());
            }
        }
        if self.authorized && self.client_id.is_none()
            || self.folder_id.is_some() && !self.authorized
            || self.folder_id.is_some() != self.folder_name.is_some()
        {
            return Err("wizard_corrupt".into());
        }
        if self.folder_id.as_ref().is_some_and(|s| !token(s))
            || self.folder_name.as_ref().is_some_and(|s| s.len() > 1024)
        {
            return Err("wizard_corrupt".into());
        }
        if let Some(b) = &self.binding {
            b.validate()?;
            if self.folder_id.as_ref() != Some(&b.folder) {
                return Err("wizard_corrupt".into());
            }
        }
        if (self.recovery_saved || self.proof_verified) && self.binding.is_none()
            || self.proof_verified && !self.recovery_saved
            || self.complete && !self.proof_verified
        {
            return Err("wizard_corrupt".into());
        }
        if self.page > self.next() {
            return Err("wizard_corrupt".into());
        }
        Ok(())
    }
}
pub struct Transaction {
    root: PathBuf,
    _lock: File,
    pub state: Wizard,
}
impl Transaction {
    pub fn open(root: &Path) -> Result<Self> {
        storage::directory(root)?;
        let lock = storage::lock(&root.join("wizard.lock"))?;
        let path = root.join("wizard.json");
        let state = match std::fs::symlink_metadata(&path) {
            Ok(_) => serde_json::from_slice(&storage::read(&path, 65536)?)
                .map_err(|_| "wizard_corrupt")?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Wizard::default(),
            Err(_) => return Err("store_unavailable".into()),
        };
        let t = Self {
            root: root.into(),
            _lock: lock,
            state,
        };
        t.state.validate()?;
        Ok(t)
    }
    pub fn save(&self) -> Result<Wizard> {
        self.state.validate()?;
        storage::replace(
            &self.root.join("wizard.json"),
            &serde_json::to_vec_pretty(&self.state).map_err(|_| "wizard_corrupt")?,
        )?;
        Ok(self.state.clone())
    }
    pub fn navigate(&mut self, mode: &str, page: usize) -> Result<Wizard> {
        if !["guided", "manual"].contains(&mode) || page > self.state.next() {
            return Err("wizard_step_required".into());
        }
        self.state.mode = mode.into();
        self.state.page = page;
        self.save()
    }
    pub fn client(&mut self, id: String, source: &str) -> Result<Wizard> {
        if self.state.client_id.as_ref() != Some(&id)
            || self.state.client_source.as_deref() != Some(source)
        {
            // Keep earlier drafts recoverable when replacing the authorization context.
            self.archive()?;
            let mode = self.state.mode.clone();
            self.state = Wizard::default();
            self.state.mode = mode;
        }
        self.state.client_id = Some(id);
        self.state.client_source = Some(source.into());
        self.state.page = self.state.next();
        self.save()
    }
    pub fn check_account(&self, account: &super::drive::Account) -> Result<()> {
        account.validate()?;
        if self
            .state
            .account
            .as_ref()
            .is_some_and(|saved| saved.permission_id != account.permission_id)
        {
            return Err("account_mismatch".into());
        }
        Ok(())
    }
    pub fn accept_account(&mut self, account: super::drive::Account) -> Result<Wizard> {
        self.check_account(&account)?;
        if self.state.client_id.is_none() {
            return Err("wizard_step_required".into());
        }
        self.state.account = Some(account);
        self.authorized()
    }
    pub fn authorized(&mut self) -> Result<Wizard> {
        if self.state.client_id.is_none() {
            return Err("wizard_step_required".into());
        }
        self.state.authorized = true;
        self.state.page = self.state.next();
        self.save()
    }
    pub fn folder(&mut self, id: String, name: String) -> Result<Wizard> {
        if !self.state.authorized {
            return Err("wizard_step_required".into());
        }
        if self.state.folder_id.as_ref() != Some(&id) {
            if self.state.binding.is_some() {
                return Err("wizard_restart_required".into());
            }
            self.state.complete = false;
            self.state.folder_id = Some(id);
            self.state.folder_name = Some(name);
        }
        self.state.page = self.state.next();
        self.save()
    }
    pub fn operation_root(&self) -> Result<PathBuf> {
        let p = self.root.join(format!("wizard-{}", self.state.session));
        storage::directory(&p)?;
        Ok(p)
    }
    fn archive(&self) -> Result<()> {
        let p = self.root.join("wizard-history");
        storage::directory(&p)?;
        storage::immutable(
            &p.join(format!("{}.json", uuid::Uuid::new_v4())),
            &serde_json::to_vec_pretty(&self.state).map_err(|_| "wizard_corrupt")?,
        )?;
        Ok(())
    }
    pub fn prepare(&mut self, remote: &impl Objects, vault: &impl SecretStore) -> Result<Wizard> {
        let folder = self.state.folder_id.clone().ok_or("wizard_step_required")?;
        if self.state.binding.is_none() {
            let space = uuid::Uuid::new_v4().to_string();
            let proof = remote.allocate()?;
            let binding = Binding {
                folder,
                space,
                proof,
            };
            binding.validate()?;
            save_space_key(vault, &binding.space, &SpaceKey::generate()?)?;
            self.state.binding = Some(binding);
            self.state.complete = false;
        } else {
            load_space_key(
                vault,
                &self
                    .state
                    .binding
                    .as_ref()
                    .ok_or("wizard_step_required")?
                    .space,
            )?;
        }
        self.save()
    }
    pub fn recovery(&self, vault: &impl SecretStore) -> Result<Zeroizing<Vec<u8>>> {
        let binding = self.state.binding.clone().ok_or("wizard_step_required")?;
        let key = load_space_key(vault, &binding.space)?;
        let kit = RecoveryKit {
            schema: 1,
            binding,
            key: key.recovery_code().to_string(),
        };
        Ok(Zeroizing::new(
            serde_json::to_vec_pretty(&kit).map_err(|_| "invalid_recovery_kit")?,
        ))
    }
    pub fn recovery_exported(&mut self) -> Result<Wizard> {
        if self.state.binding.is_none() {
            return Err("wizard_step_required".into());
        }
        self.state.recovery_saved = true;
        self.state.page = self.state.next();
        self.save()
    }
    pub fn publish(&mut self, remote: &impl Objects, vault: &impl SecretStore) -> Result<Wizard> {
        if !self.state.recovery_saved {
            return Err("recovery_backup_required".into());
        }
        let b = self.state.binding.as_ref().ok_or("wizard_step_required")?;
        let key = load_space_key(vault, &b.space)?;
        let proof = proof_bundle(&b.space)?;
        match remote.put(&b.folder, &b.proof, &key, &proof) {
            Ok(()) => {}
            Err(e) if e == "drive_id_exists" => {}
            Err(e) => return Err(e),
        }
        if remote.get(&b.folder, &b.proof, &b.space, &key)? != proof {
            return Err("invalid_space_proof".into());
        }
        self.state.proof_verified = true;
        self.state.page = 4;
        self.save()
    }
    pub fn import(
        &mut self,
        bytes: &[u8],
        remote: &impl Objects,
        vault: &impl SecretStore,
    ) -> Result<Wizard> {
        let kit = RecoveryKit::parse(bytes)?;
        if self.state.folder_id.as_ref() != Some(&kit.binding.folder) {
            return Err("recovery_wrong_folder".into());
        }
        if self
            .state
            .binding
            .as_ref()
            .is_some_and(|b| *b != kit.binding)
        {
            return Err("wizard_restart_required".into());
        }
        let key = SpaceKey::recover(&kit.key)?;
        if remote.get(
            &kit.binding.folder,
            &kit.binding.proof,
            &kit.binding.space,
            &key,
        )? != proof_bundle(&kit.binding.space)?
        {
            return Err("invalid_space_proof".into());
        }
        save_space_key(vault, &kit.binding.space, &key)?;
        self.state.binding = Some(kit.binding.clone());
        self.state.recovery_saved = true;
        self.state.proof_verified = true;
        self.state.complete = false;
        self.state.page = 4;
        self.save()
    }
    pub fn finish(&mut self, remote: &impl Objects, vault: &impl SecretStore) -> Result<Wizard> {
        if !self.state.recovery_saved || !self.state.proof_verified {
            return Err("wizard_step_required".into());
        }
        let b = self.state.binding.as_ref().ok_or("wizard_step_required")?;
        let key = load_space_key(vault, &b.space)?;
        if remote.get(&b.folder, &b.proof, &b.space, &key)? != proof_bundle(&b.space)? {
            return Err("invalid_space_proof".into());
        }
        self.state.complete = true;
        self.state.page = 4;
        self.save()
    }
}
/// Explicit restart also works on malformed JSON; preserve original bytes before replacing anything.
pub fn restart(root: &Path) -> Result<Wizard> {
    storage::directory(root)?;
    let _lock = storage::lock(&root.join("wizard.lock"))?;
    let path = root.join("wizard.json");
    match std::fs::symlink_metadata(&path) {
        Ok(_) => {
            let original = storage::read(&path, 65536)?;
            let history = root.join("wizard-history");
            storage::directory(&history)?;
            storage::immutable(
                &history.join(format!("{}.json", uuid::Uuid::new_v4())),
                &original,
            )?;
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err("store_unavailable".into()),
    }
    let state = Wizard::default();
    storage::replace(
        &path,
        &serde_json::to_vec_pretty(&state).map_err(|_| "wizard_corrupt")?,
    )?;
    Ok(state)
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryKit {
    pub schema: u32,
    pub binding: Binding,
    pub key: String,
}
impl Drop for RecoveryKit {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        self.key.zeroize();
    }
}
impl RecoveryKit {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > 16384 {
            return Err("invalid_recovery_kit".into());
        }
        let k: Self = serde_json::from_slice(bytes).map_err(|_| "invalid_recovery_kit")?;
        if k.schema != 1 {
            return Err("invalid_recovery_kit".into());
        }
        k.binding.validate()?;
        SpaceKey::recover(&k.key)?;
        Ok(k)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        cell::{Cell, RefCell},
        collections::BTreeMap,
    };
    #[test]
    fn account_binding_survives_reload_and_rejects_switch_without_changing_progress() {
        let d = tempfile::tempdir().unwrap();
        let mut t = Transaction::open(d.path()).unwrap();
        t.client("fixture.apps.googleusercontent.com".into(), "imported")
            .unwrap();
        let mut account = super::super::drive::Account {
            permission_id: "123".into(),
            display_name: Some("Cat".into()),
            email_address: None,
        };
        t.accept_account(account.clone()).unwrap();
        t.folder("folder".into(), "Bastet".into()).unwrap();
        drop(t);
        let mut t = Transaction::open(d.path()).unwrap();
        let before = t.state.clone();
        account.permission_id = "456".into();
        assert_eq!(
            t.accept_account(account.clone()).unwrap_err(),
            "account_mismatch"
        );
        assert_eq!(t.state, before);
        account.permission_id = "123".into();
        account.display_name = Some("Renamed".into());
        t.accept_account(account).unwrap();
        assert_eq!(t.state.folder_id.as_deref(), Some("folder"));
        drop(t);
        assert_eq!(
            Transaction::open(d.path())
                .unwrap()
                .state
                .account
                .unwrap()
                .display_name
                .as_deref(),
            Some("Renamed")
        );
    }
    #[test]
    fn old_wizard_progress_without_account_still_loads() {
        let d = tempfile::tempdir().unwrap();
        let state = Wizard::default();
        let mut raw = serde_json::to_value(state).unwrap();
        raw.as_object_mut().unwrap().remove("account");
        storage::replace(
            &d.path().join("wizard.json"),
            &serde_json::to_vec(&raw).unwrap(),
        )
        .unwrap();
        assert!(Transaction::open(d.path()).unwrap().state.account.is_none());
    }
    #[derive(Default)]
    struct Vault(RefCell<BTreeMap<String, String>>);
    impl SecretStore for Vault {
        fn read(&self, a: &str) -> Result<Option<Zeroizing<String>>> {
            Ok(self.0.borrow().get(a).cloned().map(Zeroizing::new))
        }
        fn write(&self, a: &str, s: &str) -> Result<()> {
            self.0.borrow_mut().insert(a.into(), s.into());
            Ok(())
        }
        fn remove(&self, a: &str) -> Result<()> {
            self.0.borrow_mut().remove(a);
            Ok(())
        }
    }
    #[derive(Default)]
    struct Remote {
        data: RefCell<BTreeMap<String, Vec<u8>>>,
        fail: Cell<bool>,
        allocated: Cell<usize>,
    }
    impl Objects for Remote {
        fn ids(&self, _: &str) -> Result<Vec<String>> {
            Ok(self.data.borrow().keys().cloned().collect())
        }
        fn allocate(&self) -> Result<String> {
            let n = self.allocated.get();
            self.allocated.set(n + 1);
            Ok(format!("proof-{n}"))
        }
        fn put(
            &self,
            _: &str,
            id: &str,
            key: &SpaceKey,
            b: &crate::sync::bundle::Bundle,
        ) -> Result<()> {
            if self.data.borrow().contains_key(id) {
                return Err("drive_id_exists".into());
            }
            self.data.borrow_mut().insert(id.into(), key.seal(b)?);
            if self.fail.replace(false) {
                Err("network_unavailable".into())
            } else {
                Ok(())
            }
        }
        fn get(
            &self,
            _: &str,
            id: &str,
            space: &str,
            key: &SpaceKey,
        ) -> Result<crate::sync::bundle::Bundle> {
            key.open(space, self.data.borrow().get(id).ok_or("drive_not_found")?)
        }
    }
    fn folder_ready(path: &Path) -> Transaction {
        let mut t = Transaction::open(path).unwrap();
        t.client("fixture.apps.googleusercontent.com".into(), "imported")
            .unwrap();
        t.authorized().unwrap();
        t.folder("folder".into(), "Fixture".into()).unwrap();
        t
    }
    #[test]
    fn resume_all_completed_steps_without_recreating_key_or_proof() {
        let d = tempfile::tempdir().unwrap();
        let remote = Remote::default();
        let vault = Vault::default();
        let mut t = Transaction::open(d.path()).unwrap();
        t.navigate("manual", 0).unwrap();
        drop(t);
        let mut t = Transaction::open(d.path()).unwrap();
        assert_eq!(t.state.mode, "manual");
        t.client("fixture.apps.googleusercontent.com".into(), "imported")
            .unwrap();
        drop(t);
        let mut t = Transaction::open(d.path()).unwrap();
        assert_eq!(t.state.next(), 1);
        t.authorized().unwrap();
        drop(t);
        let mut t = Transaction::open(d.path()).unwrap();
        assert_eq!(t.state.next(), 2);
        t.folder("folder".into(), "Fixture".into()).unwrap();
        t.prepare(&remote, &vault).unwrap();
        let binding = t.state.binding.clone();
        drop(t);
        let mut t = Transaction::open(d.path()).unwrap();
        t.prepare(&remote, &vault).unwrap();
        assert_eq!(binding, t.state.binding);
        assert_eq!(remote.allocated.get(), 1);
        assert!(t.publish(&remote, &vault).is_err());
        assert!(remote.data.borrow().is_empty());
        let kit = t.recovery(&vault).unwrap();
        t.recovery_exported().unwrap();
        drop(t);
        let mut t = Transaction::open(d.path()).unwrap();
        remote.fail.set(true);
        assert!(t.publish(&remote, &vault).is_err());
        assert!(!t.state.proof_verified);
        drop(t);
        let mut t = Transaction::open(d.path()).unwrap();
        t.publish(&remote, &vault).unwrap();
        t.finish(&remote, &vault).unwrap();
        drop(t);
        let t = Transaction::open(d.path()).unwrap();
        assert!(t.state.complete);
        assert_eq!(remote.data.borrow().len(), 1);
        let saved = std::fs::read_to_string(d.path().join("wizard.json")).unwrap();
        let recovery = RecoveryKit::parse(&kit).unwrap();
        assert!(!saved.contains(&recovery.key));
    }
    #[test]
    fn manual_mode_cannot_bypass_validation_or_change_bound_folder() {
        let d = tempfile::tempdir().unwrap();
        let mut t = Transaction::open(d.path()).unwrap();
        t.navigate("manual", 0).unwrap();
        assert!(t.navigate("guided", 4).is_err());
        assert!(t.folder("folder".into(), "X".into()).is_err());
        assert!(t.finish(&Remote::default(), &Vault::default()).is_err());
        drop(t);
        let mut t = folder_ready(d.path());
        t.prepare(&Remote::default(), &Vault::default()).unwrap();
        assert!(t.folder("other".into(), "Y".into()).is_err());
    }
    #[test]
    fn joining_verifies_recovery_before_persisting_key_and_survives_restart() {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        let remote = Remote::default();
        let vault = Vault::default();
        let other_vault = Vault::default();
        let mut t = folder_ready(a.path());
        t.prepare(&remote, &vault).unwrap();
        let kit = t.recovery(&vault).unwrap();
        t.recovery_exported().unwrap();
        t.publish(&remote, &vault).unwrap();
        let mut joined = folder_ready(b.path());
        let mut changed = RecoveryKit::parse(&kit).unwrap();
        changed.key = SpaceKey::generate().unwrap().recovery_code().to_string();
        assert!(joined
            .import(
                &serde_json::to_vec(&changed).unwrap(),
                &remote,
                &other_vault
            )
            .is_err());
        assert!(other_vault.0.borrow().is_empty());
        assert!(joined.state.binding.is_none());
        joined.import(&kit, &remote, &other_vault).unwrap();
        joined.finish(&remote, &other_vault).unwrap();
        drop(joined);
        assert!(Transaction::open(b.path()).unwrap().state.complete);
    }
    #[test]
    fn restart_archives_progress_and_malformed_state_without_deleting_secrets() {
        let d = tempfile::tempdir().unwrap();
        let mut t = folder_ready(d.path());
        let vault = Vault::default();
        t.prepare(&Remote::default(), &vault).unwrap();
        let session = t.state.session.clone();
        drop(t);
        let fresh = restart(d.path()).unwrap();
        assert_ne!(session, fresh.session);
        assert_eq!(fresh.next(), 0);
        assert_eq!(vault.0.borrow().len(), 1);
        let history = d.path().join("wizard-history");
        let before = std::fs::read_dir(&history).unwrap().count();
        std::fs::write(d.path().join("wizard.json"), b"{broken").unwrap();
        assert!(Transaction::open(d.path()).is_err());
        restart(d.path()).unwrap();
        assert_eq!(std::fs::read_dir(history).unwrap().count(), before + 1);
    }
}
