use crate::files;
use anyhow::Result;
use serde_json::Value;
use std::{fs, io, path::Path};

/// Preserve the first completed receipt while retaining each explicit experiment.
pub(super) fn consumed(package: &Path, seal: &str, receipt: &Value) -> Result<()> {
    files::atomic_json(
        &package.join("consumption").join(format!("{seal}.json")),
        receipt,
    )?;
    let first = package.join("consumed.json");
    let temporary = package.join(format!(".consumed-{}.partial", uuid::Uuid::now_v7()));
    files::atomic_json(&temporary, receipt)?;
    let result = match fs::hard_link(&temporary, &first) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(error.into()),
    };
    fs::remove_file(temporary)?;
    result
}
