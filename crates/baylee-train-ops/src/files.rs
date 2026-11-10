//! Durable metadata and streaming input seals.
use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::Value;
use std::{
    fs::{self, File, OpenOptions, TryLockError},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
    process::{Child, Command},
    sync::{PoisonError, RwLock},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

/// Held shared by every [`spawn`] and exclusively by a [`try_lock`] that found
/// its file busy.
///
/// A child holds a copy of each of this process's descriptors from its fork
/// until its `exec` closes them, and an advisory lock belongs to the open file
/// description, not to the descriptor. A lock that one thread releases while
/// another thread starts a child therefore stays held by that child for a
/// moment, and a new handle on the same file is refused as if a second owner
/// existed.
static SPAWNING: RwLock<()> = RwLock::new(());

/// How long a busy lock is retried once no child can be starting.
///
/// A child's `exec` closes its copies right after the parent's spawn returns;
/// this only bounds a starved scheduler. A real owner keeps the lock and is
/// still refused.
const INHERITED_RELEASE: Duration = Duration::from_millis(100);

/// Start a child process. Every spawn in the library goes through here, so that
/// [`try_lock`] can tell a starting child's inherited copy of a lock from its
/// owner.
/// # Errors
/// Returns the spawn error.
pub fn spawn(command: &mut Command) -> std::io::Result<Child> {
    let _spawning = SPAWNING.read().unwrap_or_else(PoisonError::into_inner);
    command.spawn()
}

/// Take an exclusive advisory lock without waiting for its owner.
///
/// A busy lock is retried, for at most [`INHERITED_RELEASE`], after every
/// spawn in this process has returned and while new ones are held back, so a
/// lock still held only by a starting child's inherited descriptor is not
/// mistaken for another owner.
/// # Errors
/// Returns [`TryLockError::WouldBlock`] while another owner holds the lock,
/// or the I/O error.
pub fn try_lock(file: &File) -> Result<(), TryLockError> {
    match file.try_lock() {
        Err(TryLockError::WouldBlock) => {}
        result => return result,
    }
    let _quiet = SPAWNING.write().unwrap_or_else(PoisonError::into_inner);
    let deadline = Instant::now() + INHERITED_RELEASE;
    loop {
        match file.try_lock() {
            Err(TryLockError::WouldBlock) if Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(1));
            }
            result => return result,
        }
    }
}

/// Return wall clock time for operational receipts (never engine decisions).
/// # Errors
/// Returns an error when the system clock predates the Unix epoch.
pub fn unix_time() -> Result<f64> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs_f64())
}

/// Read a JSON receipt. The file is read whole and parsed from memory: a bare
/// `File` under `serde_json::from_reader` costs one system call per byte.
/// # Errors
/// Rejects missing, unreadable or malformed files.
pub fn read_json(path: &Path) -> Result<Value> {
    let bytes = fs::read(path).with_context(|| format!("open {}", path.display()))?;
    serde_json::from_slice(&bytes).with_context(|| format!("read {}", path.display()))
}

fn output_parent(path: &Path) -> Result<&Path> {
    let parent = path.parent().context("output needs a parent")?;
    Ok(if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    })
}

/// Publish a flushed JSON file by atomic rename, then sync its parent directory.
/// # Errors
/// Returns serialization or filesystem errors, removing the temporary on failure.
pub fn atomic_json(path: &Path, value: &impl Serialize) -> Result<()> {
    publish(path, |file| {
        serde_json::to_writer_pretty(&mut *file, value)?;
        file.write_all(b"\n")?;
        Ok(())
    })
}

/// Publish a flushed text file as [`atomic_json`] publishes JSON.
/// # Errors
/// Returns filesystem errors, removing the temporary on failure.
pub fn atomic_text(path: &Path, text: &str) -> Result<()> {
    publish(path, |file| Ok(file.write_all(text.as_bytes())?))
}

/// Write a temporary beside `path`, flush it, rename it over `path` and sync
/// the parent directory; the temporary is removed on failure.
fn publish(path: &Path, write: impl FnOnce(&mut File) -> Result<()>) -> Result<()> {
    let parent = output_parent(path)?;
    fs::create_dir_all(parent)?;
    let tmp = parent.join(format!(
        ".{}.{}.partial",
        path.file_name()
            .context("output filename")?
            .to_string_lossy(),
        uuid::Uuid::now_v7()
    ));
    let result = (|| -> Result<()> {
        let mut file = OpenOptions::new().write(true).create_new(true).open(&tmp)?;
        write(&mut file)?;
        file.sync_all()?;
        fs::rename(&tmp, path)?;
        #[cfg(unix)]
        File::open(parent)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(tmp);
    }
    result
}

/// Append one serialized metric record under an exclusive file lock.
/// # Errors
/// Rejects a torn previous line, concurrent writers and I/O failures. A failed
/// append attempts to restore the original length instead of leaving a prefix.
pub fn append_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let mut line = serde_json::to_vec(value)?;
    line.push(b'\n');
    let parent = output_parent(path)?;
    fs::create_dir_all(parent)?;
    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(path)?;
    try_lock(&file).context("metric log already owned")?;
    let length = file.metadata()?.len();
    if length > 0 {
        file.seek(SeekFrom::End(-1))?;
        let mut last = [0];
        file.read_exact(&mut last)?;
        anyhow::ensure!(last == *b"\n", "previous metric record is incomplete");
    }
    if let Err(error) = file.write_all(&line).and_then(|()| file.sync_all()) {
        file.set_len(length)
            .context("restore failed metric append")?;
        file.sync_all()?;
        return Err(error.into());
    }
    #[cfg(unix)]
    File::open(parent)?.sync_all()?;
    Ok(())
}

/// Stream a SHA256 digest while checking that the input did not change.
/// # Errors
/// Rejects unreadable, non-regular, changed or replaced files.
pub fn sha256(path: &Path) -> Result<String> {
    crate::digests::InputDigestCache::default().sha256(path)
}

/// Publish a byte-identical artifact without exposing a partial replacement.
/// # Errors
/// Rejects unreadable or changing sources and preserves the old destination on failure.
pub fn atomic_copy(source: &Path, destination: &Path) -> Result<String> {
    let digest = sha256(source)?;
    let parent = output_parent(destination)?;
    fs::create_dir_all(parent)?;
    let temporary = parent.join(format!(".{}.partial", uuid::Uuid::now_v7()));
    let result = (|| -> Result<String> {
        let mut input = File::open(source)?;
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        std::io::copy(&mut input, &mut output)?;
        output.sync_all()?;
        anyhow::ensure!(
            sha256(&temporary)? == digest && sha256(source)? == digest,
            "artifact changed during publication"
        );
        fs::rename(&temporary, destination)?;
        #[cfg(unix)]
        File::open(parent)?.sync_all()?;
        Ok(digest)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

/// Encode bytes in lowercase hexadecimal.
#[must_use]
pub fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        result.push(char::from(DIGITS[usize::from(byte >> 4)]));
        result.push(char::from(DIGITS[usize::from(byte & 15)]));
    }
    result
}

/// Acquire an exclusive file lock until the returned handle is dropped.
/// # Errors
/// Returns I/O errors or rejects a lock already held by another controller.
pub fn lock(path: &Path) -> Result<File> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)?;
    try_lock(&file).with_context(|| format!("already owned: {}", path.display()))?;
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[cfg(unix)]
    #[test]
    fn a_starting_child_is_not_a_second_lock_owner_but_a_real_owner_is() -> Result<()> {
        let root = tempfile::tempdir()?;
        let path = root.path().join("owner.lock");
        let done = AtomicBool::new(false);
        thread::scope(|scope| -> Result<()> {
            let spawner = scope.spawn(|| -> Result<()> {
                while !done.load(Ordering::Relaxed) {
                    spawn(&mut Command::new("true"))?.wait()?;
                }
                Ok(())
            });
            // Each child starts with a copy of whatever lock is held at that
            // moment; releasing and retaking it must still succeed.
            let relocked = (0..2000).try_for_each(|_| lock(&path).map(drop));
            done.store(true, Ordering::Relaxed);
            spawner.join().expect("spawner thread")?;
            relocked
        })?;
        let owner = lock(&path)?;
        assert!(lock(&path).is_err());
        drop(owner);
        lock(&path)?;
        Ok(())
    }

    #[test]
    fn artifact_replacement_preserves_previous_output_on_failure() -> Result<()> {
        let root = tempfile::tempdir()?;
        let source = root.path().join("source");
        let out = root.path().join("published");
        fs::write(&source, b"complete new model")?;
        fs::write(&out, b"previous model")?;
        assert!(atomic_copy(&root.path().join("missing"), &out).is_err());
        assert_eq!(fs::read(&out)?, b"previous model");
        assert_eq!(atomic_copy(&source, &out)?, sha256(&source)?);
        assert_eq!(fs::read(&out)?, b"complete new model");
        assert_eq!(fs::read_dir(root.path())?.count(), 2);
        Ok(())
    }

    #[test]
    fn metric_append_preserves_history_and_refuses_a_torn_record() -> Result<()> {
        let root = tempfile::tempdir()?;
        let path = root.path().join("metrics.jsonl");
        append_json(&path, &serde_json::json!({"step": 0}))?;
        append_json(&path, &serde_json::json!({"step": 1}))?;
        assert_eq!(fs::read_to_string(&path)?, "{\"step\":0}\n{\"step\":1}\n");
        fs::write(&path, b"{\"step\":")?;
        assert!(append_json(&path, &serde_json::json!({"step": 2})).is_err());
        assert_eq!(fs::read(&path)?, b"{\"step\":");
        Ok(())
    }
}
