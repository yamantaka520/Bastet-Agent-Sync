use super::bundle::{Result, MAX_WIRE};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::Path,
};

pub fn directory(path: &Path) -> Result<()> {
    // Managed child directories must never redirect to a different store.
    match fs::symlink_metadata(path) {
        Ok(m) if m.is_dir() && !m.file_type().is_symlink() => Ok(()),
        Ok(_) => Err("unsafe_store".into()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir(path).map_err(|_| "store_unavailable".into())
        }
        Err(_) => Err("store_unavailable".into()),
    }
}
pub fn read(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let m = fs::symlink_metadata(path).map_err(|_| "store_unavailable")?;
    if !m.is_file() || m.file_type().is_symlink() {
        return Err("unsafe_store".into());
    }
    if m.len() > limit {
        return Err("bundle_limit".into());
    }
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|_| "store_unavailable")?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "store_unavailable")?;
    if bytes.len() as u64 > limit {
        return Err("bundle_limit".into());
    }
    Ok(bytes)
}
pub fn immutable(path: &Path, bytes: &[u8]) -> Result<bool> {
    if fs::symlink_metadata(path).is_ok() {
        if read(path, MAX_WIRE)? == bytes {
            return Ok(false);
        }
        return Err("immutable_collision".into());
    }
    let parent = path.parent().ok_or("unsafe_store")?;
    let mut tmp = tempfile::NamedTempFile::new_in(parent).map_err(|_| "store_unavailable")?;
    tmp.write_all(bytes).map_err(|_| "store_unavailable")?;
    tmp.as_file().sync_all().map_err(|_| "store_unavailable")?;
    match tmp.persist_noclobber(path) {
        Ok(_) => {
            sync_dir(parent)?;
            Ok(true)
        }
        Err(e) if e.error.kind() == std::io::ErrorKind::AlreadyExists => {
            if read(path, MAX_WIRE)? == bytes {
                Ok(false)
            } else {
                Err("immutable_collision".into())
            }
        }
        Err(_) => Err("store_unavailable".into()),
    }
}
pub fn replace(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Ok(m) = fs::symlink_metadata(path) {
        if !m.is_file() || m.file_type().is_symlink() {
            return Err("unsafe_store".into());
        }
    }
    let parent = path.parent().ok_or("unsafe_store")?;
    let mut tmp = tempfile::NamedTempFile::new_in(parent).map_err(|_| "store_unavailable")?;
    tmp.write_all(bytes).map_err(|_| "store_unavailable")?;
    tmp.as_file().sync_all().map_err(|_| "store_unavailable")?;
    tmp.persist(path).map_err(|_| "store_unavailable")?;
    sync_dir(parent)
}
fn sync_dir(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        File::open(path)
            .and_then(|f| f.sync_all())
            .map_err(|_| "store_unavailable")?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}
pub fn lock(path: &Path) -> Result<File> {
    if let Ok(m) = fs::symlink_metadata(path) {
        if !m.is_file() || m.file_type().is_symlink() {
            return Err("unsafe_store".into());
        }
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|_| "store_unavailable")?;
    fs2::FileExt::try_lock_exclusive(&file).map_err(|_| "sync_busy")?;
    Ok(file)
}
