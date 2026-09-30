//! The settings file on disk, on native targets: where it is, reading it,
//! and writing it whole.

use super::{FILE, SeatSettings};
use crate::userdirs::{Kind, Os, user_dir};
use std::ffi::OsString;
use std::io::Write as _;
use std::path::{Path, PathBuf};

/// Where the settings file is on `os`, reading the environment through
/// `env`: [`FILE`] in the client's config directory, or `None` where the
/// environment names none.
#[must_use]
pub fn default_path(os: Os, env: &dyn Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    Some(user_dir(Kind::Config, os, env)?.join(FILE))
}

/// The settings at `path`, or `None` when there is no file.
///
/// # Errors
/// A sentence naming the file, for one that cannot be read or that
/// [`SeatSettings::parse`] refuses.
pub fn load(path: &Path) -> Result<Option<SeatSettings>, String> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(format!(
                "the settings file {} cannot be read: {e}",
                path.display()
            ));
        }
    };
    SeatSettings::parse(&text)
        .map(Some)
        .map_err(|why| format!("the settings file {}: {why}", path.display()))
}

/// Writes `settings` to `path` whole, once [`SeatSettings::check`] passes.
///
/// # Errors
/// What `check` refuses, or why the file could not be written.
pub fn save(path: &Path, settings: &SeatSettings) -> Result<(), String> {
    settings.check()?;
    write_whole(path, settings.to_json().as_bytes()).map_err(|e| {
        format!(
            "the settings file {} cannot be written: {e}",
            path.display()
        )
    })
}

/// Writes `bytes` to `path` so that a reader finds the old file or the
/// new one, never a part: into a temporary beside it, synced, then renamed
/// over it. Readable and writable by its owner alone on unix (`0600`): it
/// is nobody else's business what a player spends.
pub(super) fn write_whole(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent().filter(|dir| !dir.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir)?;
    }
    let temporary = path.with_extension("tmp");
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let mut file = options.open(&temporary)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    std::fs::rename(&temporary, path)?;
    // The rename is only as durable as the directory that records it.
    #[cfg(unix)]
    if let Some(dir) = path.parent().filter(|dir| !dir.as_os_str().is_empty()) {
        std::fs::File::open(dir)?.sync_all()?;
    }
    Ok(())
}
