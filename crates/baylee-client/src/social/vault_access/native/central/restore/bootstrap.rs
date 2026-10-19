//! One-time legacy trust import and bounded handoff of an exclusively locked local vault.
use super::{
    ChatDevice, ChatDisk, ContextGuard, Error, Handle, MemoryDisk, Scope, Session, Social,
    ensure_running, folder, social, sync,
};
use std::{
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};

pub(super) fn open_context(
    root: &Path,
    scope: Scope,
    cancelled: &AtomicBool,
) -> Result<ContextGuard, Error> {
    wait_for_context(root, scope, cancelled, Duration::from_secs(15))
}

fn wait_for_context(
    root: &Path,
    scope: Scope,
    cancelled: &AtomicBool,
    timeout: Duration,
) -> Result<ContextGuard, Error> {
    let deadline = Instant::now() + timeout;
    loop {
        ensure_running(cancelled)?;
        match ContextGuard::open(root, scope.clone()) {
            Err(Error::Busy) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(100));
            }
            result => return result,
        }
    }
}

pub(super) fn archive(
    handle: &Handle,
    device: &ChatDevice,
    local: Option<&ChatDevice>,
    root: &Path,
    context: &ContextGuard,
    session: &Session,
) -> Result<MemoryDisk, Error> {
    // The remote read must succeed even for an initial import. Unavailability,
    // corruption, or an existing remote root must never trigger a local fallback.
    let remote = MemoryDisk {
        files: handle.chat_history(device.clone()).map_err(|_| Error::Io)?,
    };
    if local.is_none_or(|local| local.device != device.device || local.key != device.key) {
        return Ok(remote);
    }
    let initial = local_initial(
        device,
        [
            folder(context, device),
            context.directory().join("chat"),
            root.join(social::disk::chat_folder(&session.gateway, &session.tag)),
        ],
    )?;
    Ok(initial.unwrap_or(remote))
}

fn local_initial(
    device: &ChatDevice,
    paths: impl IntoIterator<Item = PathBuf>,
) -> Result<Option<MemoryDisk>, Error> {
    if device.history.is_some() {
        return Ok(None);
    }
    let keys = baylee_seal::DeviceSecrets::from_bytes(device.key.bytes());
    for path in paths {
        let archive = MemoryDisk {
            files: sync::snapshot(&path)?,
        };
        if let Some(state) = archive.read(social::disk::STATE_FILE) {
            Social::validate_stored_state(&keys, &state).map_err(|_| Error::Authentication)?;
            return Ok(Some(archive));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests;
