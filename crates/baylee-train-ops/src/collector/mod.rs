//! Collector lifecycle tools. No gateway mutation or implicit collector restart.
mod arm;
pub mod launch;
pub mod monitor;
pub use arm::arm;
mod probe;
pub mod repair;
mod transition;
use crate::files;
use anyhow::{Context, Result, ensure};
pub use probe::validate_clean_probe;
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
};
pub use transition::{finish, resume};

fn text<'a>(v: &'a Value, key: &str) -> Result<&'a str> {
    v[key]
        .as_str()
        .with_context(|| format!("missing string {key}"))
}
fn json(path: impl AsRef<Path>) -> Result<Value> {
    files::read_json(path.as_ref())
}
fn pins(value: &Value) -> Result<()> {
    let object = value.as_object().context("input seals must be an object")?;
    ensure!(!object.is_empty(), "input seals required");
    for (path, digest) in object {
        let expected = digest.as_str().context("invalid input seal")?;
        ensure!(
            expected.len() == 64
                && expected
                    .bytes()
                    .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)),
            "invalid SHA256"
        );
        ensure!(
            files::sha256(Path::new(path))? == expected,
            "pinned input changed"
        );
    }
    Ok(())
}
fn attempts(root: &Path, name: &str) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    let directory = root.join("attempts");
    if directory.exists() {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            ensure!(
                !entry.file_type()?.is_symlink(),
                "symlink attempt directory refused"
            );
            let path = entry.path().join(name);
            if path.is_file() {
                paths.push(path);
            }
        }
    }
    paths.sort();
    Ok(paths)
}
fn verify_closed(root: &Path) -> Result<()> {
    fn blocked(path: &Path) -> bool {
        [
            "transport-blocked",
            "transport-blocked.json",
            "TRANSPORT_BLOCKED",
        ]
        .iter()
        .any(|name| path.join(name).exists())
    }
    ensure!(!blocked(root), "transport blocker exists");
    for request in attempts(root, "request.json")? {
        let directory = request.parent().context("attempt parent")?;
        ensure!(!blocked(directory), "attempt transport blocker exists");
        let room = json(directory.join("room-state.json"))?;
        ensure!(
            room["status"] == "closed" && room["closure_verified"] == true,
            "attempt closure unverified"
        );
    }
    Ok(())
}

fn try_lock(path: &Path) -> Result<Option<std::fs::File>> {
    match files::lock(path) {
        Ok(lock) => Ok(Some(lock)),
        Err(error)
            if error.chain().any(|cause| {
                cause
                    .downcast_ref::<std::fs::TryLockError>()
                    .is_some_and(|e| matches!(e, std::fs::TryLockError::WouldBlock))
            }) =>
        {
            Ok(None)
        }
        Err(error) => Err(error),
    }
}
