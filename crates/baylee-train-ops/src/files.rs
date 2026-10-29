//! Durable metadata and streaming input seals.
use std::{fs::{self, File, OpenOptions}, io::{Read, Write}, path::Path, time::{SystemTime, UNIX_EPOCH}};
use anyhow::{Context, Result, ensure};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

/// Return wall clock time for operational receipts (never engine decisions).
/// # Errors
/// Returns an error when the system clock predates the Unix epoch.
pub fn unix_time() -> Result<f64> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs_f64())
}

/// Read a JSON receipt.
/// # Errors
/// Rejects missing, unreadable or malformed files.
pub fn read_json(path: &Path) -> Result<Value> {
    serde_json::from_reader(File::open(path).with_context(|| format!("open {}", path.display()))?)
        .with_context(|| format!("read {}", path.display()))
}

/// Publish a flushed JSON file by atomic rename, then sync its parent directory.
/// # Errors
/// Returns serialization or filesystem errors, removing the temporary on failure.
pub fn atomic_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let parent = path.parent().context("output needs a parent")?;
    fs::create_dir_all(parent)?;
    let tmp = parent.join(format!(".{}.{}.partial", path.file_name().context("output filename")?.to_string_lossy(), uuid::Uuid::now_v7()));
    let result = (|| -> Result<()> {
        let mut file = OpenOptions::new().write(true).create_new(true).open(&tmp)?;
        serde_json::to_writer_pretty(&mut file, value)?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        fs::rename(&tmp, path)?;
        #[cfg(unix)]
        File::open(parent)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() { let _ = fs::remove_file(tmp); }
    result
}

/// Stream a SHA256 digest while checking that the input did not change.
/// # Errors
/// Rejects unreadable, non-regular, changed or replaced files.
pub fn sha256(path: &Path) -> Result<String> {
    let mut file = File::open(path).with_context(|| format!("hash {}", path.display()))?;
    let before = file.metadata()?;
    ensure!(before.is_file(), "input must be a regular file: {}", path.display());
    let mut digest = Sha256::new();
    let mut buf = vec![0; 1024 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 { break; }
        digest.update(&buf[..n]);
    }
    let after = file.metadata()?;
    ensure!(before.len() == after.len() && before.modified()? == after.modified()?, "input changed during hashing");
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let current = fs::metadata(path)?;
        ensure!((before.dev(), before.ino(), before.ctime(), before.ctime_nsec()) ==
            (after.dev(), after.ino(), after.ctime(), after.ctime_nsec()) &&
            (after.dev(), after.ino(), after.ctime(), after.ctime_nsec()) ==
            (current.dev(), current.ino(), current.ctime(), current.ctime_nsec()), "input replaced during hashing");
    }
    Ok(hex(&digest.finalize()))
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
    if let Some(parent) = path.parent() { fs::create_dir_all(parent)?; }
    let file = OpenOptions::new().create(true).truncate(false).read(true).write(true).open(path)?;
    file.try_lock().with_context(|| format!("already owned: {}", path.display()))?;
    Ok(file)
}
