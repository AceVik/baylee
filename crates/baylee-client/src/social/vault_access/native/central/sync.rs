//! Bounded ciphertext snapshots. Filesystem work and Central uploads run on the account worker.
#![forbid(unsafe_code)]
use super::super::{Error, social};
use super::{ChatDevice, ContextGuard, Handle};
use social::disk::{ChatDisk, DirDisk};
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

pub(crate) struct SyncState {
    disk: Mutex<DirDisk>,
    batch: Mutex<()>,
    _context: Arc<ContextGuard>,
    folder: std::path::PathBuf,
    device: ChatDevice,
    handle: Handle,
    changed: AtomicU64,
    uploaded: AtomicU64,
    pending: AtomicBool,
    failed: AtomicBool,
    local_failed: AtomicBool,
    last_attempt: Mutex<Option<Instant>>,
    complete: (Mutex<()>, std::sync::Condvar),
}

impl SyncState {
    pub(super) fn new(
        context: Arc<ContextGuard>,
        folder: std::path::PathBuf,
        device: ChatDevice,
        handle: Handle,
    ) -> Arc<Self> {
        Arc::new(Self {
            disk: Mutex::new(DirDisk::new(&folder)),
            batch: Mutex::new(()),
            _context: context,
            folder,
            device,
            handle,
            changed: AtomicU64::new(1),
            uploaded: AtomicU64::new(1),
            pending: AtomicBool::new(false),
            failed: AtomicBool::new(false),
            local_failed: AtomicBool::new(false),
            last_attempt: Mutex::new(None),
            complete: (Mutex::new(()), std::sync::Condvar::new()),
        })
    }

    pub(crate) fn generation(&self) -> u64 {
        self.changed.load(Ordering::Acquire)
    }

    pub(crate) fn begin_batch(&self) -> Option<std::sync::MutexGuard<'_, ()>> {
        if let Ok(guard) = self.batch.lock() {
            Some(guard)
        } else {
            self.local_failed.store(true, Ordering::Release);
            None
        }
    }

    fn capture(&self) -> Result<(u64, BTreeMap<String, Vec<u8>>), Error> {
        let _batch = self.batch.lock().map_err(|_| Error::Io)?;
        let _disk = self.disk.lock().map_err(|_| Error::Io)?;
        if self.local_failed.load(Ordering::Acquire) {
            return Err(Error::Io);
        }
        Ok((
            self.changed.load(Ordering::Acquire),
            snapshot(&self.folder)?,
        ))
    }

    pub(crate) fn acknowledged(&self) -> Option<u64> {
        (!self.local_failed.load(Ordering::Acquire) && self.handle.is_active())
            .then(|| self.uploaded.load(Ordering::Acquire))
    }

    pub(crate) fn status(&self) -> crate::social::vault_access::SyncStatus {
        use crate::social::vault_access::SyncStatus;
        if self.failed.load(Ordering::Acquire) || self.local_failed.load(Ordering::Acquire) {
            SyncStatus::Failed
        } else if self.changed.load(Ordering::Acquire) == self.uploaded.load(Ordering::Acquire) {
            SyncStatus::Saved
        } else {
            SyncStatus::Pending
        }
    }

    pub(crate) fn queue(self: &Arc<Self>, force: bool) {
        if self.local_failed.load(Ordering::Acquire) {
            return;
        }
        if self.changed.load(Ordering::Acquire) == self.uploaded.load(Ordering::Acquire) {
            return;
        }
        let Ok(mut last) = self.last_attempt.lock() else {
            self.failed.store(true, Ordering::Release);
            return;
        };
        if !force && last.is_some_and(|time| time.elapsed() < Duration::from_secs(5)) {
            return;
        }
        if self
            .pending
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return;
        }
        *last = Some(Instant::now());
        drop(last);
        let state = self.clone();
        if !self.handle.submit(move |account| {
            let result = (|| {
                // A snapshot is consistent with a completed local disk operation. Never hold
                // this lock during network/KDF work, which must not block rendering.
                let (generation, files) = state.capture()?;
                account
                    .upload_chat_history(&state.device, &files)
                    .map_err(|_| Error::Io)?;
                state.uploaded.store(generation, Ordering::Release);
                Ok::<(), Error>(())
            })();
            state.failed.store(result.is_err(), Ordering::Release);
            state.pending.store(false, Ordering::Release);
            state.complete.1.notify_all();
            // Writes racing the network upload need a second snapshot, including on logout
            // when this job may be the last owner of the account worker's sender.
            if result.is_ok() {
                state.queue(true);
            }
        }) {
            self.pending.store(false, Ordering::Release);
            self.failed.store(true, Ordering::Release);
        }
    }

    fn drain(self: &Arc<Self>) {
        if std::thread::panicking() {
            return;
        }
        self.queue(true);
        let until = Instant::now() + Duration::from_secs(30);
        let Ok(mut guard) = self.complete.0.lock() else {
            return;
        };
        while self.changed.load(Ordering::Acquire) != self.uploaded.load(Ordering::Acquire) {
            if !self.handle.is_active() {
                break;
            }
            if !self.pending.load(Ordering::Acquire)
                && (self.failed.load(Ordering::Acquire)
                    || self.local_failed.load(Ordering::Acquire))
            {
                break;
            }
            let remaining = until.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }
            let Ok((next, _)) = self
                .complete
                .1
                .wait_timeout(guard, remaining.min(Duration::from_secs(1)))
            else {
                return;
            };
            guard = next;
        }
    }
}

pub(super) struct SyncDisk(pub(super) Arc<SyncState>);
impl SyncDisk {
    fn change(
        &self,
        operation: impl FnOnce(&mut DirDisk) -> Result<(), String>,
    ) -> Result<(), String> {
        let result = self
            .0
            .disk
            .lock()
            .map_err(|_| Error::Io.to_string())
            .and_then(|mut disk| {
                let result = operation(&mut disk);
                if result.is_ok() {
                    self.0.changed.fetch_add(1, Ordering::Release);
                } else {
                    self.0.local_failed.store(true, Ordering::Release);
                }
                result
            });
        if result.is_err() {
            // Do not publish a partial local snapshot or later call it synchronized.
            // Restart performs strict authentication/recovery before writing again.
            self.0.local_failed.store(true, Ordering::Release);
        }
        result
    }
}
impl Drop for SyncDisk {
    fn drop(&mut self) {
        self.0.drain();
    }
}
impl ChatDisk for SyncDisk {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        self.0.disk.lock().ok()?.read(path)
    }
    fn list(&self, dir: &str) -> Vec<String> {
        self.0
            .disk
            .lock()
            .map_or_else(|_| Vec::new(), |disk| disk.list(dir))
    }
    fn write(&mut self, path: &str, bytes: &[u8]) -> Result<(), String> {
        self.change(|disk| disk.write(path, bytes))
    }
    fn append(&mut self, path: &str, bytes: &[u8]) -> Result<(), String> {
        self.change(|disk| disk.append(path, bytes))
    }
    fn remove(&mut self, path: &str) -> Result<(), String> {
        self.change(|disk| disk.remove(path))
    }
    fn remove_all(&mut self) -> Result<(), String> {
        self.change(ChatDisk::remove_all)
    }
}

pub(super) fn snapshot(folder: &Path) -> Result<BTreeMap<String, Vec<u8>>, Error> {
    check_directory(folder)?;
    check_directory(&folder.join(social::disk::HISTORY_DIR))?;
    let mut files = BTreeMap::new();
    let state = folder.join(social::disk::STATE_FILE);
    if state.try_exists().map_err(|_| Error::Io)? {
        files.insert(
            social::disk::STATE_FILE.into(),
            super::super::bounded_read(&state)?,
        );
    }
    match std::fs::read_dir(folder.join(social::disk::HISTORY_DIR)) {
        Ok(entries) => {
            let mut total = files.values().map(Vec::len).sum::<usize>();
            for (at, entry) in entries.take(1025).enumerate() {
                if at == 1024 {
                    return Err(Error::Format);
                }
                let entry = entry.map_err(|_| Error::Io)?;
                if !entry.file_type().map_err(|_| Error::Io)?.is_file() {
                    return Err(Error::Io);
                }
                let name = entry.file_name().into_string().map_err(|_| Error::Format)?;
                if name.starts_with('.') {
                    continue;
                }
                let Some(stem) = name.strip_suffix(".log") else {
                    return Err(Error::Format);
                };
                if stem.len() != 16
                    || !stem
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
                {
                    return Err(Error::Format);
                }
                let bytes = super::super::bounded_read(&entry.path())?;
                total += bytes.len();
                if total > 256 * 1024 * 1024 {
                    return Err(Error::Format);
                }
                files.insert(social::disk::history_path(&name), bytes);
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err(Error::Io),
    }
    if !files.is_empty() && !files.contains_key(social::disk::STATE_FILE) {
        return Err(Error::Format);
    }
    Ok(files)
}

fn check_directory(path: &Path) -> Result<(), Error> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) => {
            if !meta.is_dir() || meta.file_type().is_symlink() {
                return Err(Error::Io);
            }
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt as _;
                if meta.file_attributes() & 0x400 != 0 {
                    return Err(Error::Io);
                }
            }
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(Error::Io),
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::{OsEntropy, Scope};
    use super::*;
    use baylee_account::chat::DeviceKey;
    use std::sync::mpsc;
    #[test]
    fn local_write_failure_blocks_publication_and_delivery_ack() {
        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let root = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("baylee-chat-failed-write-{}", uuid::Uuid::now_v7()));
        let _cleanup = Cleanup(root.clone());
        std::fs::create_dir(&root).unwrap();
        let context = Arc::new(
            ContextGuard::open(
                &root,
                Scope::new("af03", "https://gateway.example").unwrap(),
            )
            .unwrap(),
        );
        let keys = baylee_seal::DeviceSecrets::generate(&mut OsEntropy);
        let device = ChatDevice {
            gateway: "https://gateway.example".into(),
            tag: "af03".into(),
            installation: context.installation().unwrap(),
            device: keys.device_id().into(),
            key: DeviceKey::new(keys.to_bytes()),
            retired: false,
            history: None,
        };
        let folder = context.directory().join("test-chat");
        let state = SyncState::new(context, folder, device, Handle::unavailable_for_test());
        let mut disk = SyncDisk(state.clone());
        assert_eq!(state.acknowledged(), Some(1));
        let batch = state.begin_batch().unwrap();
        disk.append("history/0123456789abcdef.log", b"synthetic ciphertext")
            .unwrap();
        let snapshot_state = state.clone();
        let (started, starting) = mpsc::channel();
        let (done, completed) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            started.send(()).unwrap();
            done.send(snapshot_state.capture()).unwrap();
        });
        starting.recv().unwrap();
        let blocked = completed.recv_timeout(Duration::from_millis(100)).is_err();
        disk.write("state.bin", b"synthetic state ciphertext")
            .unwrap();
        drop(batch);
        let (generation, files) = completed
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
            .unwrap();
        worker.join().unwrap();
        assert!(blocked, "a partial effect batch must not publish");
        assert_eq!(generation, state.generation());
        assert_eq!(files.len(), 2);
        assert!(disk.write("../escape", b"synthetic ciphertext").is_err());
        assert_eq!(
            state.acknowledged(),
            None,
            "an earlier durable generation must not hide a failed current write"
        );
        state.queue(true);
        assert!(!state.pending.load(Ordering::Acquire));
        assert!(state.status() == crate::social::vault_access::SyncStatus::Failed);
        assert!(!root.join("escape").exists());
    }

    #[test]
    fn snapshot_requires_a_complete_bounded_chat_namespace() {
        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let root =
            std::env::temp_dir().join(format!("baylee-chat-snapshot-{}", uuid::Uuid::now_v7()));
        let _cleanup = Cleanup(root.clone());
        std::fs::create_dir_all(root.join("history")).unwrap();
        std::fs::write(
            root.join("history/0123456789abcdef.log"),
            b"synthetic ciphertext",
        )
        .unwrap();
        assert!(
            snapshot(&root).is_err(),
            "logs without their conversation mapping must not publish"
        );
        std::fs::write(root.join("state.bin"), b"synthetic state ciphertext").unwrap();
        assert_eq!(snapshot(&root).unwrap().len(), 2);
        std::fs::write(root.join("history/plaintext-or-unknown-file"), b"excluded").unwrap();
        assert!(snapshot(&root).is_err());
        std::fs::remove_file(root.join("history/plaintext-or-unknown-file")).unwrap();
        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(root.join("history/0123456789abcdef.log"))
            .unwrap();
        file.set_len(16 * 1024 * 1024 + 1).unwrap();
        assert!(snapshot(&root).is_err());
    }
}
