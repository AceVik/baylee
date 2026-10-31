//! Restore the single active logical identity before opening any chat transport.
use super::super::{
    Choice, Error, Me, OsEntropy, Ready, Scope, Session, Social, Zeroizing, legacy_secrets, now_ms,
    social,
};
use super::{CentralStore, ContextGuard, Handle, SyncState, sync};
use baylee_account::chat::{ChatDevice, DeviceKey};
use social::disk::{ChatDisk, MemoryDisk};
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

mod bootstrap;

#[allow(clippy::too_many_arguments)] // one background startup's immutable inputs
pub(super) fn start(
    root: &Path,
    session: Session,
    handle: Handle,
    choice: Choice,
    password: Zeroizing<String>,
    prefs: social::ChatPrefs,
    keep: u32,
    cancelled: &AtomicBool,
) -> Result<Ready, Error> {
    let scope = Scope::new(&session.tag, &session.gateway)?;
    let vault_scope = *scope.as_bytes();
    let context = Arc::new(bootstrap::open_context(root, scope, cancelled)?);
    let installation = context.installation()?;
    let slot = social::SecretSlot::new(&session.tag, &session.gateway);
    let mut records = handle
        .chat_devices(session.gateway.clone(), session.tag.clone())
        .map_err(|_| Error::Io)?;
    for record in &records {
        if baylee_seal::DeviceSecrets::from_bytes(record.key.bytes()).device_id() != record.device {
            return Err(Error::Format);
        }
    }
    ensure_running(cancelled)?;
    // An old local vault opens with the account's key where a password
    // sign-in gave it the wrapper (`docs/password-vault.md` §"The account key").
    let account_key = handle.local_vault_key(vault_scope).ok();
    let local = import_keys(
        root,
        &context,
        &session,
        &slot,
        &installation,
        &records,
        choice,
        account_key.as_deref(),
        &password,
    )?;
    drop(password);
    drop(account_key);
    if let Some(local) = &local {
        handle
            .archive_chat_device(local.clone())
            .map_err(|_| Error::Io)?;
        if !records.iter().any(|record| record.device == local.device) {
            records.push(local.clone());
        }
    }
    let device = select_device(&records)
        .ok_or(Error::Authentication)?
        .clone();
    let base = bootstrap::archive(&handle, &device, local.as_ref(), root, &context, &session)?;
    let (mut machine, mut disk, deferred) = start_shared(base, &device, &session, prefs, keep)?;
    machine.trust_own_devices(
        records
            .iter()
            .filter(|record| !record.retired)
            .map(|record| {
                let keys = baylee_seal::DeviceSecrets::from_bytes(record.key.bytes());
                (keys.ik(), keys.dk())
            })
            .collect(),
    );
    for record in &records {
        ensure_running(cancelled)?;
        let archived_keys = baylee_seal::DeviceSecrets::from_bytes(record.key.bytes());
        if record.device != device.device && record.history.is_some() {
            let archive = MemoryDisk {
                files: handle.chat_history(record.clone()).map_err(|_| Error::Io)?,
            };
            merge(&mut machine, &archived_keys, &archive, &mut disk)?;
        }
        // A prior session may have newer local history, but its seen/keys/protocol
        // state must never roll back the state downloaded from Central.
        merge_folder(
            &mut machine,
            &archived_keys,
            &folder(&context, record),
            &mut disk,
        )?;
    }
    if let Some(local) = &local {
        merge_legacy(&mut machine, local, root, &context, &session, &mut disk)?;
    }
    // All input is authenticated before normal retention may rewrite it, including
    // the explicit zero-history setting, which must clear old complete snapshots too.
    for effect in machine.restore(&disk, now_ms(), &mut OsEntropy) {
        social::disk::apply(&effect, &mut disk)
            .ok_or(Error::Format)?
            .map_err(|_| Error::Io)?;
    }
    validate_size(&disk)?;
    ensure_running(cancelled)?;
    let target = folder(&context, &device);
    persist(&target, &disk)?;
    handle
        .remember_chat_installation(device.clone(), installation)
        .map_err(|_| Error::Io)?;
    handle
        .upload_chat_history(device.clone(), disk.files)
        .map_err(|_| Error::Io)?;
    ensure_running(cancelled)?;
    let sync = SyncState::new(context.clone(), target, device.clone(), handle.clone());
    Ok(Ready {
        machine,
        effects: deferred,
        disk: Box::new(sync::SyncDisk(sync.clone())),
        store: Box::new(CentralStore {
            context,
            handle,
            slot,
            device,
        }),
        sync: Some(sync),
    })
}

fn merge_legacy(
    machine: &mut Social,
    local: &ChatDevice,
    root: &Path,
    context: &ContextGuard,
    session: &Session,
    disk: &mut MemoryDisk,
) -> Result<(), Error> {
    let keys = baylee_seal::DeviceSecrets::from_bytes(local.key.bytes());
    merge_folder(machine, &keys, &context.directory().join("chat"), disk)?;
    merge_folder(
        machine,
        &keys,
        &root.join(social::disk::chat_folder(&session.gateway, &session.tag)),
        disk,
    )
}

fn start_shared(
    base: MemoryDisk,
    device: &ChatDevice,
    session: &Session,
    prefs: social::ChatPrefs,
    keep: u32,
) -> Result<(Social, MemoryDisk, Vec<social::Effect>), Error> {
    let keys = baylee_seal::DeviceSecrets::from_bytes(device.key.bytes());
    let stored = base.read(social::disk::STATE_FILE);
    if device.history.is_some() && stored.is_none() {
        return Err(Error::Format);
    }
    if let Some(bytes) = &stored {
        Social::validate_stored_state(&keys, bytes).map_err(|_| Error::Authentication)?;
    }
    let (mut machine, initial) = Social::start(
        Me {
            tag: session.tag.clone(),
            handle: session.handle.clone(),
            guest: false,
            gateway: session.gateway.clone(),
        },
        stored,
        Some(social::vault::encode_secrets(&keys).to_string()),
        prefs,
        now_ms(),
        &mut OsEntropy,
    );
    machine.set_key_home(social::KeyHome::Central);
    machine.set_keep_days(keep);
    let mut disk = base.clone();
    let mut deferred = Vec::new();
    // SaveState from start must precede imported maps; network stays closed until every
    // source authenticates and the resulting manifest has been acknowledged centrally.
    for effect in initial {
        if let Some(result) = social::disk::apply(&effect, &mut disk) {
            result.map_err(|_| Error::Io)?;
        } else {
            deferred.push(effect);
        }
    }
    merge(&mut machine, &keys, &base, &mut disk)?;
    Ok((machine, disk, deferred))
}
/// Stable account-wide selection; newly discovered installations never fork this identity.
fn select_device(records: &[ChatDevice]) -> Option<&ChatDevice> {
    records.iter().find(|record| !record.retired)
}

#[allow(clippy::too_many_arguments)] // bounded local import, with explicit old-vault credentials
fn import_keys(
    root: &Path,
    context: &ContextGuard,
    session: &Session,
    slot: &social::SecretSlot,
    installation: &str,
    records: &[ChatDevice],
    choice: Choice,
    account_key: Option<&[u8; 32]>,
    password: &str,
) -> Result<Option<ChatDevice>, Error> {
    if let Some(record) = records
        .iter()
        .find(|record| record.installation == installation)
    {
        return Ok(Some(record.clone()));
    }
    let legacy = root.join(social::disk::chat_folder(&session.gateway, &session.tag));
    let keys = match context.legacy_keys(account_key, (!password.is_empty()).then_some(password))? {
        Some(keys) => Some(baylee_seal::DeviceSecrets::from_bytes(&keys)),
        None if choice == Choice::ImportFile
            || legacy
                .join(social::vault::KEY_FILE)
                .try_exists()
                .map_err(|_| Error::Io)? =>
        {
            Some(legacy_secrets(
                &social::vault::FileSecrets::new(legacy.join(social::vault::KEY_FILE)),
                slot,
            )?)
        }
        None if choice == Choice::ImportKeychain
            || legacy.try_exists().map_err(|_| Error::Io)? =>
        {
            Some(legacy_secrets(&social::vault::OsSecrets, slot)?)
        }
        None => {
            if context
                .directory()
                .join("chat")
                .try_exists()
                .map_err(|_| Error::Io)?
            {
                return Err(Error::Missing);
            }
            if records.is_empty() {
                Some(baylee_seal::DeviceSecrets::generate(&mut OsEntropy))
            } else {
                None
            }
        }
    };
    let Some(keys) = keys else {
        return Ok(None);
    };
    if let Some(known) = records
        .iter()
        .find(|record| record.device == keys.device_id())
    {
        if known.key.bytes().as_slice() != keys.to_bytes().as_slice() {
            return Err(Error::Authentication);
        }
        return Ok(Some(known.clone()));
    }
    Ok(Some(ChatDevice {
        // Filed where `Handle::chat_devices` looks, and where an installed
        // client on another device looks too.
        gateway: baylee_client_core::lobby::gateway_list::keyed(&session.gateway).to_string(),
        tag: session.tag.clone(),
        installation: installation.into(),
        device: keys.device_id().into(),
        key: DeviceKey::new(keys.to_bytes()),
        retired: false,
        history: None,
    }))
}

fn folder(context: &ContextGuard, device: &ChatDevice) -> std::path::PathBuf {
    context
        .directory()
        .join(format!("central-{}.chat", device.device))
}

fn ensure_running(cancelled: &AtomicBool) -> Result<(), Error> {
    if cancelled.load(Ordering::Acquire) {
        Err(Error::Busy)
    } else {
        Ok(())
    }
}

fn merge(
    machine: &mut Social,
    keys: &baylee_seal::DeviceSecrets,
    archive: &MemoryDisk,
    current: &mut MemoryDisk,
) -> Result<(), Error> {
    let effects = machine
        .restore_archive(keys, archive, current, now_ms(), &mut OsEntropy)
        .map_err(|_| Error::Authentication)?;
    for effect in effects {
        social::disk::apply(&effect, current)
            .ok_or(Error::Format)?
            .map_err(|_| Error::Io)?;
    }
    validate_size(current)?;
    Ok(())
}

fn validate_size(disk: &MemoryDisk) -> Result<(), Error> {
    if disk.files.len() > 1025
        || disk
            .files
            .values()
            .any(|bytes| bytes.len() > 16 * 1024 * 1024)
        || disk.files.values().map(Vec::len).sum::<usize>() > 256 * 1024 * 1024
    {
        return Err(Error::Format);
    }
    Ok(())
}

fn merge_folder(
    machine: &mut Social,
    keys: &baylee_seal::DeviceSecrets,
    path: &Path,
    current: &mut MemoryDisk,
) -> Result<(), Error> {
    let archive = MemoryDisk {
        files: sync::snapshot(path)?,
    };
    merge(machine, keys, &archive, current)
}

fn persist(path: &Path, source: &MemoryDisk) -> Result<(), Error> {
    let before = sync::snapshot(path)?;
    let mut target = social::disk::DirDisk::new(path);
    for (name, bytes) in &source.files {
        if name != social::disk::STATE_FILE {
            target.write(name, bytes).map_err(|_| Error::Io)?;
        }
    }
    if let Some(state) = source.files.get(social::disk::STATE_FILE) {
        target
            .write(social::disk::STATE_FILE, state)
            .map_err(|_| Error::Io)?;
    }
    for name in before
        .keys()
        .filter(|name| !source.files.contains_key(*name))
    {
        target.remove(name).map_err(|_| Error::Io)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The live gateway's chat after its move: the vault context an
    /// installed client opened under the former address is this client's
    /// (the same installation), and a first chat record is filed under the
    /// address an installed client on another device looks for.
    #[test]
    #[ignore = "the client still pins the old address; runs from the flip on (#402)"]
    fn the_live_gateway_s_chat_context_and_records_outlive_its_move() {
        use baylee_client_core::lobby::gateway_list::{KEYED, PINNED};
        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let root = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("baylee-moved-chat-{}", uuid::Uuid::now_v7()));
        let _cleanup = Cleanup(root.clone());
        std::fs::create_dir(&root).unwrap();
        let before = {
            let context = ContextGuard::open(&root, Scope::new("af03", KEYED).unwrap()).unwrap();
            context.installation().unwrap()
        };
        let session = Session {
            gateway: PINNED.into(),
            tag: "af03".into(),
            handle: "Alice#af03".into(),
            token: "synthetic token".into(),
            guest: false,
        };
        let context =
            ContextGuard::open(&root, Scope::new(&session.tag, &session.gateway).unwrap()).unwrap();
        let installation = context.installation().unwrap();
        assert_eq!(installation, before, "the same installation, not a new one");
        let slot = social::SecretSlot::new(&session.tag, &session.gateway);
        let first = import_keys(
            &root,
            &context,
            &session,
            &slot,
            &installation,
            &[],
            Choice::Unlock,
            None,
            "",
        )
        .unwrap()
        .expect("a first identity");
        assert_eq!(first.gateway, KEYED);
    }

    #[test]
    fn a_new_installation_reuses_identity_and_never_replaces_keyless_ciphertext() {
        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let root = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("baylee-shared-chat-{}", uuid::Uuid::now_v7()));
        let _cleanup = Cleanup(root.clone());
        std::fs::create_dir(&root).unwrap();
        let session = Session {
            gateway: "https://gateway.example".into(),
            tag: "af03".into(),
            handle: "Alice#af03".into(),
            token: "synthetic token".into(),
            guest: false,
        };
        let scope = Scope::new(&session.tag, &session.gateway).unwrap();
        let context = ContextGuard::open(&root, scope).unwrap();
        let installation = context.installation().unwrap();
        let slot = social::SecretSlot::new(&session.tag, &session.gateway);
        let keys = baylee_seal::DeviceSecrets::generate(&mut OsEntropy);
        let device = ChatDevice {
            gateway: session.gateway.clone(),
            tag: session.tag.clone(),
            installation: uuid::Uuid::now_v7().to_string(),
            device: keys.device_id().into(),
            key: DeviceKey::new(keys.to_bytes()),
            retired: false,
            history: None,
        };
        let records = vec![device];
        assert!(
            import_keys(
                &root,
                &context,
                &session,
                &slot,
                &installation,
                &records,
                Choice::Unlock,
                None,
                ""
            )
            .unwrap()
            .is_none()
        );
        assert_eq!(select_device(&records).unwrap().device, keys.device_id());
        let mut imported = records[0].clone();
        imported.installation = "00000000-0000-7000-8000-000000000001".into();
        imported.device = "later-import-from-older-installation".into();
        let mut expanded = records.clone();
        expanded.push(imported);
        assert_eq!(
            select_device(&expanded).unwrap().device,
            keys.device_id(),
            "a later legacy import must not rotate the shared identity"
        );
        std::fs::create_dir(context.directory().join("chat")).unwrap();
        let ciphertext = context.directory().join("chat/state.bin");
        std::fs::write(&ciphertext, b"existing undecryptable ciphertext").unwrap();
        assert!(matches!(
            import_keys(
                &root,
                &context,
                &session,
                &slot,
                &installation,
                &[],
                Choice::Unlock,
                None,
                ""
            ),
            Err(Error::Missing)
        ));
        assert_eq!(
            std::fs::read(ciphertext).unwrap(),
            b"existing undecryptable ciphertext"
        );
    }
}
