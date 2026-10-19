//! The account worker owns Central's master key. Chat receives only its scoped device key.
#![forbid(unsafe_code)]

use super::{Choice, Error, Job, SecretStore, SocialHost, WORKER_BUSY, Zeroizing, social};
use crate::lobby::central::Handle;
use baylee_account::chat::ChatDevice;
use baylee_client_core::social::vault::password::ContextGuard;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};

mod restore;
mod sync;
pub(crate) use sync::SyncState;

struct CentralStore {
    context: Arc<ContextGuard>,
    handle: Handle,
    slot: social::SecretSlot,
    device: ChatDevice,
}

impl SecretStore for CentralStore {
    fn home(&self) -> social::KeyHome {
        social::KeyHome::Central
    }

    fn load(&self, slot: &social::SecretSlot) -> Result<Option<String>, String> {
        if *slot != self.slot {
            return Err(Error::Scope.to_string());
        }
        let device = baylee_seal::DeviceSecrets::from_bytes(self.device.key.bytes());
        Ok(Some(social::vault::encode_secrets(&device).to_string()))
    }

    fn store(&self, slot: &social::SecretSlot, value: &str) -> Result<(), String> {
        if *slot != self.slot {
            return Err(Error::Scope.to_string());
        }
        let device =
            social::vault::decode_secrets(value).ok_or_else(|| Error::Format.to_string())?;
        // Initialization already persisted this exact key before any public registration.
        // A future rotation must add a new archive record through an acknowledged worker job.
        if device.to_bytes().as_slice() != self.device.key.bytes() {
            return Err(Error::Exists.to_string());
        }
        Ok(())
    }

    fn forget(&self, slot: &social::SecretSlot) -> Result<(), String> {
        if *slot != self.slot {
            return Err(Error::Scope.to_string());
        }
        self.context.retire().map_err(|error| error.to_string())?;
        self.handle
            .retire_chat_device(self.device.clone())
            .map_err(|_| Error::Io.to_string())
    }
}

pub(super) fn begin(
    host: &mut SocialHost,
    handle: Handle,
    choice: Choice,
    password: Zeroizing<String>,
) {
    if host.vault.busy() || host.machine.is_some() {
        return;
    }
    let Some(root) = super::super::super::config_dir() else {
        return;
    };
    let Some(session) = host.session.clone().filter(|session| !session.guest) else {
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
    let launched = std::thread::Builder::new()
        .name("central-chat-restore".into())
        .spawn(move || {
            struct Done;
            impl Drop for Done {
                fn drop(&mut self) {
                    WORKER_BUSY.store(false, Ordering::Release);
                }
            }
            let _done = Done;
            let result = restore::start(
                &root,
                session,
                handle,
                choice,
                password,
                prefs,
                keep,
                &cancel_job,
            );
            let _ = sender.send(result);
        });
    if launched.is_err() {
        WORKER_BUSY.store(false, Ordering::Release);
        host.vault.job = None;
        host.vault.error = Some(Error::Io);
    }
}
