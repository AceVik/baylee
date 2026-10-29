//! Process-local file receipts. Persisted metadata never substitutes for a fresh read.
use crate::files::hex;
use anyhow::{Result, ensure};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File, Metadata},
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct Identity {
    size: u64,
    modified: std::time::SystemTime,
    #[cfg(unix)]
    inode: (u64, u64, i64, i64),
}
impl Identity {
    fn of(meta: &Metadata) -> Result<Self> {
        ensure!(meta.is_file(), "input digest requires a regular file");
        Ok(Self {
            size: meta.len(),
            modified: meta.modified()?,
            #[cfg(unix)]
            inode: {
                use std::os::unix::fs::MetadataExt;
                (meta.dev(), meta.ino(), meta.ctime(), meta.ctime_nsec())
            },
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct Receipt {
    resolved: PathBuf,
    identity: Identity,
    digest: String,
}

/// Counters for bounded disk I/O, never an authorization to trust old receipts.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Statistics {
    /// Revalidated process-memory receipt hits.
    pub hits: u64,
    /// Actual hashing attempts.
    pub hashes: u64,
    /// Input bytes read by those attempts.
    pub bytes_read: u64,
}

/// A per-controller cache which cannot be restored from serialized metadata.
#[derive(Default)]
pub struct InputDigestCache {
    receipts: BTreeMap<PathBuf, Receipt>,
    statistics: Statistics,
}

fn snapshot(path: &Path) -> Result<(PathBuf, Identity)> {
    let resolved = path.canonicalize()?;
    let identity = Identity::of(&fs::metadata(&resolved)?)?;
    ensure!(
        path.canonicalize()? == resolved && Identity::of(&fs::metadata(path)?)? == identity,
        "input changed during path resolution"
    );
    Ok((resolved, identity))
}

fn open_regular(path: &Path) -> Result<File> {
    #[cfg(target_os = "linux")]
    let file = File::from(rustix::fs::open(
        path,
        rustix::fs::OFlags::RDONLY
            | rustix::fs::OFlags::NOFOLLOW
            | rustix::fs::OFlags::NONBLOCK
            | rustix::fs::OFlags::CLOEXEC,
        rustix::fs::Mode::empty(),
    )?);
    #[cfg(not(target_os = "linux"))]
    let file = File::open(path)?;
    Identity::of(&file.metadata()?)?;
    Ok(file)
}

impl InputDigestCache {
    /// Current process-local I/O statistics.
    #[must_use]
    pub const fn statistics(&self) -> Statistics {
        self.statistics
    }

    /// Compute or revalidate a streaming SHA256 seal.
    /// # Errors
    /// Rejects non-regular files, symlink retargeting and concurrent replacement/editing.
    pub fn sha256(&mut self, path: &Path) -> Result<String> {
        self.hash_with(path, || {})
    }

    fn hash_with(&mut self, path: &Path, mut after_read: impl FnMut()) -> Result<String> {
        let requested = std::path::absolute(path)?;
        let (resolved, identity) = snapshot(&requested)?;
        if let Some(old) = self.receipts.get(&requested)
            && old.resolved == resolved
            && old.identity == identity
        {
            ensure!(
                snapshot(&requested)? == (resolved, identity),
                "input changed during cache lookup"
            );
            self.statistics.hits += 1;
            return Ok(old.digest.clone());
        }
        self.statistics.hashes += 1;
        let mut file = open_regular(&resolved)?;
        ensure!(
            Identity::of(&file.metadata()?)? == identity,
            "input replaced before hashing"
        );
        let mut digest = Sha256::new();
        let mut buffer = vec![0; 1024 * 1024];
        loop {
            let count = file.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            digest.update(&buffer[..count]);
            self.statistics.bytes_read += u64::try_from(count)?;
            after_read();
        }
        ensure!(
            Identity::of(&file.metadata()?)? == identity,
            "input changed during hashing"
        );
        ensure!(
            snapshot(&requested)? == (resolved.clone(), identity.clone()),
            "input replaced during hashing"
        );
        let digest = hex(&digest.finalize());
        self.receipts.insert(
            requested,
            Receipt {
                resolved,
                identity,
                digest: digest.clone(),
            },
        );
        Ok(digest)
    }

    /// Check caller-owned expected seals without changing them.
    /// # Errors
    /// Rejects any changed file or a digest mismatch.
    pub fn verify(&mut self, expected: &BTreeMap<PathBuf, String>) -> Result<()> {
        for (path, seal) in expected {
            ensure!(
                self.sha256(path)? == *seal,
                "sealed input digest mismatch: {}",
                path.display()
            );
        }
        Ok(())
    }
}

fn source_files(
    root: &Path,
    path: &Path,
    files: &mut std::collections::BTreeSet<PathBuf>,
) -> Result<()> {
    let resolved = path.canonicalize()?;
    ensure!(resolved.starts_with(root), "source input escapes root");
    if path.is_dir() {
        ensure!(
            !path.symlink_metadata()?.is_symlink(),
            "source directories cannot be symlinks"
        );
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() || entry.path().extension().is_some_and(|e| e == "py") {
                source_files(root, &entry.path(), files)?;
            }
        }
    } else {
        ensure!(path.is_file(), "source input must be a regular file");
        files.insert(path.to_path_buf());
    }
    Ok(())
}

/// Hash selected tensor implementation sources with relative names and separators.
/// Directories contribute their Python sources recursively, never bytecode caches.
/// # Errors
/// Rejects missing inputs, escaping symlinks or concurrent source mutations.
pub fn source_digest(root: &Path, includes: &[PathBuf]) -> Result<String> {
    let root = root.canonicalize()?;
    ensure!(!includes.is_empty(), "source inputs required");
    let mut paths = std::collections::BTreeSet::new();
    for name in includes {
        ensure!(
            name.components()
                .all(|c| matches!(c, std::path::Component::Normal(_))),
            "source path must be relative"
        );
        source_files(&root, &root.join(name), &mut paths)?;
    }
    ensure!(!paths.is_empty(), "empty source selection");
    let mut snapshots = BTreeMap::new();
    let mut digest = Sha256::new();
    let mut buffer = vec![0; 1024 * 1024];
    for path in &paths {
        let (resolved, identity) = snapshot(path)?;
        let mut file = open_regular(&resolved)?;
        ensure!(
            Identity::of(&file.metadata()?)? == identity,
            "source changed before read"
        );
        digest.update(
            path.strip_prefix(&root)?
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("source name is not UTF8"))?
                .as_bytes(),
        );
        digest.update(b"\0");
        loop {
            let n = file.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            digest.update(&buffer[..n]);
        }
        digest.update(b"\0");
        ensure!(
            Identity::of(&file.metadata()?)? == identity,
            "source changed during read"
        );
        snapshots.insert(path, (resolved, identity));
    }
    for (path, old) in snapshots {
        ensure!(
            snapshot(path)? == old,
            "source changed during manifest hashing"
        );
    }
    let mut current = std::collections::BTreeSet::new();
    for name in includes {
        source_files(&root, &root.join(name), &mut current)?;
    }
    ensure!(paths == current, "source file selection changed");
    Ok(hex(&digest.finalize()))
}

#[cfg(test)]
mod tests;
