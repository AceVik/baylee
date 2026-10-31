//! Native password-vault jobs and lossless legacy migration.

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};

use super::{Access, LobbyState, SocialHost, Zeroizing, social};
use crate::social::{ChatHome, InfoState, Me, OsEntropy, Session, Social, now_ms};
pub(crate) use baylee_client_core::social::vault::password::Error;
use baylee_client_core::social::vault::{
    SecretStore,
    password::{PasswordStore, Scope},
};

static WORKER_BUSY: AtomicBool = AtomicBool::new(false);

pub(crate) mod central;

pub(crate) struct Ready {
    machine: Social,
    effects: Vec<social::Effect>,
    disk: ChatHome,
    store: Box<dyn SecretStore>,
    sync: Option<Arc<central::SyncState>>,
}

pub(crate) struct Job {
    receiver: Mutex<mpsc::Receiver<Result<Ready, Error>>>,
    cancelled: Arc<AtomicBool>,
}

impl Drop for Job {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
    }
}

/// Each creation/import is an explicit user choice, never a load fallback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Choice {
    Unlock,
    Create,
    ImportFile,
    ImportKeychain,
}

impl Access {
    pub(crate) fn busy(&self) -> bool {
        self.job.is_some()
    }
}

pub(crate) fn follow(host: &mut SocialHost, lobby: &mut LobbyState) {
    if let Some(sync) = &host.vault.sync {
        sync.queue(false);
    }
    host.vault.central = lobby.central_handle();
    let central_start = host.vault.central.is_some()
        && !host.vault.attempted
        && host.machine.is_none()
        && host.session.as_ref().is_some_and(|session| !session.guest)
        && matches!(
            host.info,
            InfoState::Known(Some((baylee_protocol::social::SOCIAL_VERSION, _)))
        );
    follow_with(host, lobby, |host, login| {
        begin(host, Choice::Unlock, login.password, login.previous, None);
    });
    if central_start && !host.vault.busy() && host.machine.is_none() && host.vault.error.is_none() {
        begin(
            host,
            Choice::Unlock,
            Zeroizing::new(String::new()),
            None,
            None,
        );
    }
}

// Inject the I/O boundary so lifecycle tests cannot accidentally use the real
// process configuration directory when another test has opened Settings.
fn follow_with(
    host: &mut SocialHost,
    lobby: &mut LobbyState,
    start: impl FnOnce(&mut SocialHost, super::Login),
) {
    // A password endpoint may retain the same session token. Its accepted
    // credentials still require a fresh unlock/rewrap, even without end().
    if host.machine.is_some()
        && lobby.vault_login.as_ref().is_some_and(|login| {
            host.session.as_ref().is_some_and(|session| {
                login.is_for(&session.gateway) && login.token.as_deref() == Some(&session.token)
            })
        })
    {
        host.flush_save();
        host.socket.hang();
        host.socket = crate::social::Socket::default();
        host.mailbox = crate::social::Mailbox::default();
        host.machine = None;
        host.store = None;
        host.disk = None;
        host.vault.clear_sync();
        host.table = None;
        host.vault.attempted = false;
    }
    if let Some(job) = host.vault.job.as_ref() {
        let result = job.receiver.lock().ok().and_then(|r| match r.try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Disconnected) => Some(Err(Error::Io)),
            Err(mpsc::TryRecvError::Empty) => None,
        });
        if let Some(result) = result {
            host.vault.job = None;
            match result {
                Ok(mut ready) => {
                    // Preferences can change while the worker derives the key.
                    // Publish the machine with current settings, not its snapshot.
                    if ready.machine.prefs() != &host.vault.prefs {
                        ready.effects.extend(ready.machine.act(
                            social::Intent::SetPrefs(host.vault.prefs.clone()),
                            now_ms(),
                            &mut OsEntropy,
                        ));
                    }
                    host.machine = Some(ready.machine);
                    host.store = Some(ready.store);
                    host.disk = Some(ready.disk);
                    host.vault.sync = ready.sync;
                    host.vault.error = None;
                    host.carry_out(ready.effects);
                }
                Err(error) => host.vault.error = Some(error),
            }
        }
    }
    let Some(session) = host.session.as_ref() else {
        return;
    };
    if session.guest {
        lobby.vault_login = None;
        return;
    }
    if matches!(host.info, InfoState::Known(None)) {
        lobby.vault_login = None;
        return;
    }
    if !matches!(
        host.info,
        InfoState::Known(Some((baylee_protocol::social::SOCIAL_VERSION, _)))
    ) {
        return;
    }
    if host.machine.is_some() || host.vault.attempted {
        return;
    }
    host.vault.attempted = true;
    let login = lobby
        .vault_login
        .take()
        .filter(|l| l.is_for(&session.gateway) && l.token.as_deref() == Some(&session.token));
    if let Some(login) = login {
        start(host, login);
    }
}

pub(crate) fn begin(
    host: &mut SocialHost,
    choice: Choice,
    password: Zeroizing<String>,
    previous: Option<Zeroizing<String>>,
    rewrap: Option<Zeroizing<String>>,
) {
    if let Some(handle) = host.vault.central.clone() {
        central::begin(host, handle, choice, password);
        return;
    }
    if host.vault.busy() || host.machine.is_some() {
        return;
    }
    let Some(root) = super::super::config_dir() else {
        return;
    };
    let Some(session) = host.session.clone().filter(|s| !s.guest) else {
        return;
    };
    if WORKER_BUSY
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        host.vault.error = Some(Error::Busy);
        return;
    }
    let prefs = host.vault.prefs.clone();
    let keep = host.keep_days;
    let cancelled = Arc::new(AtomicBool::new(false));
    let cancel_job = cancelled.clone();
    let (sender, receiver) = mpsc::channel();
    host.vault.attempted = true;
    host.vault.error = None;
    host.vault.job = Some(Job {
        receiver: Mutex::new(receiver),
        cancelled,
    });
    std::thread::spawn(move || {
        struct Done;
        impl Drop for Done {
            fn drop(&mut self) {
                WORKER_BUSY.store(false, Ordering::Release);
            }
        }
        let _done = Done;
        let result = prepare(
            &root,
            &session,
            choice,
            &password,
            previous.as_deref().map(String::as_str),
            rewrap.as_deref().map(String::as_str),
            &cancel_job,
        )
        .and_then(|(disk, store)| {
            if cancel_job.load(Ordering::Acquire) {
                return Err(Error::Busy);
            }
            let slot = social::SecretSlot::new(&session.tag, &session.gateway);
            let secrets = store.load(&slot).map_err(|_| Error::Io)?;
            let stored = disk.read(social::disk::STATE_FILE);
            let me = Me {
                tag: session.tag,
                handle: session.handle,
                guest: false,
                gateway: session.gateway,
            };
            let (mut machine, mut effects) =
                Social::start(me, stored, secrets, prefs, now_ms(), &mut OsEntropy);
            machine.set_key_home(social::KeyHome::Password);
            machine.set_keep_days(keep);
            effects.extend(machine.restore(&*disk, now_ms(), &mut OsEntropy));
            Ok(Ready {
                machine,
                effects,
                disk,
                store: Box::new(store),
                sync: None,
            })
        });
        let _ = sender.send(result);
    });
}

// The network already accepted the new password before this runs. If the
// old wrapper still opens, rewrap it; a reset password is not recovery.
#[allow(clippy::too_many_arguments)] // one bounded job, no credentials in persistent state
fn prepare(
    root: &std::path::Path,
    session: &Session,
    choice: Choice,
    password: &str,
    previous: Option<&str>,
    rewrap: Option<&str>,
    cancelled: &AtomicBool,
) -> Result<(ChatHome, PasswordStore), Error> {
    let scope = Scope::new(&session.tag, &session.gateway)?;
    let slot = social::SecretSlot::new(&session.tag, &session.gateway);
    let folder = root.join(scope.folder()).join("chat");
    let legacy = root.join(social::disk::chat_folder(&session.gateway, &session.tag));
    if cancelled.load(Ordering::Acquire) {
        return Err(Error::Busy);
    }
    let mut changed_password = None;
    let store = if choice == Choice::Unlock {
        match PasswordStore::open(root, scope.clone(), slot.clone(), password, None) {
            Ok(store) => store,
            Err(Error::Authentication) if previous.is_some() => {
                let store = PasswordStore::open(
                    root,
                    scope.clone(),
                    slot.clone(),
                    previous.unwrap_or_default(),
                    None,
                )?;
                if cancelled.load(Ordering::Acquire) {
                    return Err(Error::Busy);
                }
                changed_password = Some(password);
                store
            }
            Err(error) => return Err(error),
        }
    } else {
        let secrets = match choice {
            Choice::Create => baylee_seal::DeviceSecrets::generate(&mut OsEntropy),
            Choice::ImportFile => {
                let old = social::vault::FileSecrets::new(legacy.join(social::vault::KEY_FILE));
                legacy_secrets(&old, &slot)?
            }
            Choice::ImportKeychain => {
                #[cfg(any(windows, target_os = "macos", target_os = "linux"))]
                {
                    legacy_secrets(&social::vault::OsSecrets, &slot)?
                }
                #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
                {
                    return Err(Error::Missing);
                }
            }
            Choice::Unlock => unreachable!(),
        };
        if choice == Choice::Create && folder.exists() {
            return Err(Error::Exists);
        }
        if choice != Choice::Create && legacy.join(social::disk::STATE_FILE).exists() {
            validate_state(&legacy, &secrets)?;
        }
        if cancelled.load(Ordering::Acquire) {
            return Err(Error::Busy);
        }
        let store = match PasswordStore::open(
            root,
            scope.clone(),
            slot.clone(),
            password,
            Some(&secrets.to_bytes()),
        ) {
            Ok(store) => store,
            Err(Error::Exists) if choice != Choice::Create => {
                // Resume an interrupted import only into the very same device.
                let store = PasswordStore::open(root, scope, slot.clone(), password, None)?;
                let kept = legacy_secrets(&store, &slot)?;
                if *kept.to_bytes() != *secrets.to_bytes() {
                    return Err(Error::Exists);
                }
                store
            }
            Err(error) => return Err(error),
        };
        if choice != Choice::Create {
            import_history(&legacy, &folder, &secrets)?;
        }
        store
    };
    // A damaged state must not be treated as a brand-new device by Social.
    if folder.join(social::disk::STATE_FILE).exists() {
        let value = Zeroizing::new(
            store
                .load(&slot)
                .map_err(|_| Error::Io)?
                .ok_or(Error::Missing)?,
        );
        let secrets = social::vault::decode_secrets(&value).ok_or(Error::Format)?;
        validate_state(&folder, &secrets)?;
    }
    if let Some(new_password) = rewrap.or(changed_password) {
        if cancelled.load(Ordering::Acquire) {
            return Err(Error::Busy);
        }
        store.rewrap(new_password)?;
    }
    Ok((Box::new(social::disk::DirDisk::new(folder)), store))
}

fn legacy_secrets(
    store: &dyn SecretStore,
    slot: &social::SecretSlot,
) -> Result<baylee_seal::DeviceSecrets, Error> {
    let value = Zeroizing::new(
        store
            .load(slot)
            .map_err(|_| Error::Io)?
            .ok_or(Error::Missing)?,
    );
    social::vault::decode_secrets(&value).ok_or(Error::Format)
}

fn bounded_read(path: &std::path::Path) -> Result<Vec<u8>, Error> {
    use std::io::Read as _;
    let meta = std::fs::symlink_metadata(path).map_err(|_| Error::Io)?;
    if !meta.is_file() || meta.len() > 16 * 1024 * 1024 {
        return Err(Error::Format);
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|_| Error::Io)?
        .take(16 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::Io)?;
    if bytes.len() > 16 * 1024 * 1024 {
        return Err(Error::Format);
    }
    Ok(bytes)
}

fn validate_state(
    folder: &std::path::Path,
    secrets: &baylee_seal::DeviceSecrets,
) -> Result<(), Error> {
    let bytes = bounded_read(&folder.join(social::disk::STATE_FILE))?;
    Social::validate_stored_state(secrets, &bytes).map_err(|_| Error::Authentication)?;
    Ok(())
}

fn import_history(
    from: &std::path::Path,
    to: &std::path::Path,
    secrets: &baylee_seal::DeviceSecrets,
) -> Result<(), Error> {
    use social::disk::ChatDisk as _;
    if !from.exists() {
        return Ok(());
    }
    if from.join(social::disk::STATE_FILE).exists() {
        validate_state(from, secrets)?;
    }
    let mut target = social::disk::DirDisk::new(to);
    let mut logs = Vec::new();
    match std::fs::read_dir(from.join(social::disk::HISTORY_DIR)) {
        Ok(entries) => {
            for (at, entry) in entries.take(1025).enumerate() {
                if at == 1024 {
                    return Err(Error::Format);
                }
                let entry = entry.map_err(|_| Error::Io)?;
                if !entry.file_type().map_err(|_| Error::Io)?.is_file() {
                    return Err(Error::Io);
                }
                let name = entry.file_name().into_string().map_err(|_| Error::Format)?;
                if !name.starts_with('.') {
                    logs.push(name);
                }
            }
            if logs.len() > 1024 {
                return Err(Error::Format);
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err(Error::Io),
    }
    logs.sort();
    let paths = std::iter::once(social::disk::STATE_FILE.to_string())
        .chain(logs.into_iter().map(|n| social::disk::history_path(&n)));
    let mut total = 0usize;
    for path in paths {
        if !from.join(&path).exists() {
            continue;
        }
        let bytes = bounded_read(&from.join(&path))?;
        total += bytes.len();
        if total > 256 * 1024 * 1024 {
            return Err(Error::Format);
        }
        if to.join(&path).exists() {
            if bounded_read(&to.join(&path))? != bytes {
                return Err(Error::Exists);
            }
        } else {
            target.write(&path, &bytes).map_err(|_| Error::Io)?;
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests;
