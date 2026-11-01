//! Native durable storage. A process lock covers the entire unlocked session;
//! ciphertext updates use a fresh 0600 file, fsync, rename and directory fsync.
//! Legacy key files and keychain entries are never modified here.

use std::fs::{File, OpenOptions};
use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use super::{
    Error, FILE_NAME, MISS_FILE, MISS_LEN, RECORD_LEN, Scope, Vault, WRAP_FILE, WRAP_LEN,
    is_miss_marker, miss_marker,
};
use crate::social::vault::{KeyHome, SecretSlot, SecretStore, decode_secrets, encode_secrets};

/// An unlocked account vault, with exclusive process ownership until dropped.
/// All entry points may do disk I/O: the client runs them on its vault worker.
pub struct PasswordStore {
    path: PathBuf,
    slot: SecretSlot,
    vault: Mutex<Vault>,
    _lock: File,
}

/// Exclusive private local context for account-encrypted chat keys.
/// Only a nonsecret installation identifier is persisted by this adapter.
pub struct ContextGuard {
    dir: PathBuf,
    scope: Scope,
    _lock: File,
}

impl ContextGuard {
    /// Lock the same context as the legacy vault without decrypting or modifying it.
    ///
    /// # Errors
    /// Unsafe filesystem context or an existing owner of the vault lock.
    pub fn open(root: &Path, scope: Scope) -> Result<Self, Error> {
        let (dir, lock) = lock_directory(root, &scope)?;
        Ok(Self {
            dir,
            scope,
            _lock: lock,
        })
    }

    /// Get or durably create this installation's public `UUIDv7`.
    ///
    /// # Errors
    /// Damaged existing identifiers are never replaced with a new identity.
    pub fn installation(&self) -> Result<String, Error> {
        if check_file(&self.dir.join("central-retired"))? {
            return Err(Error::Authentication);
        }
        let path = self.dir.join("central-installation.id");
        if check_file(&path)? {
            let mut bytes = Vec::with_capacity(37);
            options()
                .read(true)
                .open(&path)
                .map_err(|_| Error::Io)?
                .take(37)
                .read_to_end(&mut bytes)
                .map_err(|_| Error::Io)?;
            let value = String::from_utf8(bytes).map_err(|_| Error::Format)?;
            let id = uuid::Uuid::parse_str(&value).map_err(|_| Error::Format)?;
            if id.get_version_num() != 7 || id.to_string() != value {
                return Err(Error::Format);
            }
            return Ok(value);
        }
        let value = uuid::Uuid::now_v7().to_string();
        write_record(&path, value.as_bytes())?;
        Ok(value)
    }

    /// Import existing password-encrypted keys without changing or removing
    /// their source: with the Central account's key where the vault's
    /// wrapper opens with it, else with the password. A wrapper that cannot
    /// be checked, read or opened (another version, a damaged or restored
    /// copy, another account's) counts as none. The password then also
    /// writes the wrapper when the account's key is at hand
    /// (`account-key.wrap`), so the next sign-in needs no password; that
    /// write is best effort and never fails the import.
    ///
    /// # Errors
    /// Missing credentials, wrong passwords and damaged vault files fail closed.
    pub fn legacy_keys(
        &self,
        account_key: Option<&[u8; 32]>,
        password: Option<&str>,
    ) -> Result<Option<zeroize::Zeroizing<[u8; 96]>>, Error> {
        let path = self.dir.join(FILE_NAME);
        if !check_file(&path)? {
            return Ok(None);
        }
        let record = read_exact(&path, RECORD_LEN)?;
        let wrap = self.dir.join(WRAP_FILE);
        let by_account = account_key.and_then(|key| {
            let bytes = read_checked(&wrap, WRAP_LEN)?;
            Vault::unlock_with_account(self.scope.clone(), key, &bytes, &record).ok()
        });
        let vault = if let Some(vault) = by_account {
            vault
        } else {
            let vault = Vault::unlock(
                self.scope.clone(),
                password.ok_or(Error::Authentication)?,
                &record,
            )?;
            if let Some(key) = account_key {
                let _ = vault
                    .account_wrap(key)
                    .and_then(|bytes| write_record(&wrap, &bytes));
            }
            vault
        };
        let bytes = vault.secrets()?;
        let mut keys = zeroize::Zeroizing::new([0; 96]);
        if bytes.len() != keys.len() {
            return Err(Error::Format);
        }
        keys.copy_from_slice(&bytes);
        Ok(Some(keys))
    }

    /// Private account directory; the guard must outlive every active disk/store adapter.
    #[must_use]
    pub fn directory(&self) -> &Path {
        &self.dir
    }

    /// Record local revocation even if the remote archive is temporarily unavailable.
    ///
    /// # Errors
    /// Failure to durably write the tombstone; callers must keep the device stopped.
    pub fn retire(&self) -> Result<(), Error> {
        write_record(&self.dir.join("central-retired"), b"retired\n")
    }
}

/// At a Central sign-in with the password, while it is still in hand:
/// every local vault that password opens gains the account's wrapper
/// (`account-key.wrap`, from `key_for` and the vault's scope), so a later
/// sign-in without the password (a pairing) still opens it. The vault files
/// are not changed. A vault the account's key already opens costs nothing,
/// and neither does one this account's password already failed to open in
/// its present form (`account-key.miss`, written at that failure); a
/// wrapper of another account, or of the vault before it was made again, is
/// replaced. A vault another process holds is skipped. At most `limit`
/// vaults are tried, since each try costs one Argon2id run; the rest are
/// reached at the next sign-in. Answers how many gained a wrapper.
pub fn attach_account_keys(
    root: &Path,
    password: &str,
    mut key_for: impl FnMut(&Scope) -> Option<zeroize::Zeroizing<[u8; 32]>>,
    limit: usize,
) -> usize {
    let Ok(entries) = std::fs::read_dir(root.join("vault")) else {
        return 0;
    };
    let mut scopes: Vec<Scope> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| Scope::from_folder_name(entry.file_name().to_str()?))
        .collect();
    scopes.sort();
    let mut tried = 0;
    let mut attached = 0;
    for scope in scopes {
        if tried == limit {
            break;
        }
        let Ok((dir, _lock)) = lock_directory(root, &scope) else {
            continue;
        };
        let path = dir.join(FILE_NAME);
        if !matches!(check_file(&path), Ok(true)) {
            continue;
        }
        let Ok(record) = read_exact(&path, RECORD_LEN) else {
            continue;
        };
        let Some(key) = key_for(&scope) else {
            continue;
        };
        let (wrap, miss) = (dir.join(WRAP_FILE), dir.join(MISS_FILE));
        let current = read_checked(&wrap, WRAP_LEN).is_some_and(|bytes| {
            Vault::unlock_with_account(scope.clone(), &key, &bytes, &record).is_ok()
        });
        let missed = read_checked(&miss, MISS_LEN)
            .is_some_and(|bytes| is_miss_marker(&scope, &key, &record, &bytes));
        if current || missed {
            continue;
        }
        tried += 1;
        match Vault::unlock(scope.clone(), password, &record) {
            Ok(vault) => {
                if vault
                    .account_wrap(&key)
                    .and_then(|bytes| write_record(&wrap, &bytes))
                    .is_ok()
                {
                    attached += 1;
                }
            }
            Err(Error::Authentication) => {
                let _ = miss_marker(&scope, &key, &record)
                    .and_then(|bytes| write_record(&miss, &bytes));
            }
            Err(_) => {}
        }
    }
    attached
}

impl PasswordStore {
    /// Unlocks an existing vault or explicitly creates one from `initial` secrets.
    /// An existing file is never overwritten by a creation request.
    ///
    /// # Errors
    /// Unsafe permissions/symlinks, another process, damaged vault, wrong password.
    pub fn open(
        root: &Path,
        scope: Scope,
        slot: SecretSlot,
        password: &str,
        initial: Option<&[u8; 96]>,
    ) -> Result<Self, Error> {
        let (dir, lock) = lock_directory(root, &scope)?;
        let path = dir.join(FILE_NAME);
        let exists = check_file(&path)?;
        let vault = match (exists, initial) {
            (true, Some(_)) => return Err(Error::Exists),
            (false, None) => return Err(Error::Missing),
            (true, None) => Vault::unlock(scope, password, &read_exact(&path, RECORD_LEN)?)?,
            (false, Some(secrets)) => {
                let vault = Vault::create(scope, password, secrets)?;
                write_record(&path, vault.record())?;
                vault
            }
        };
        Ok(Self {
            path,
            slot,
            vault: Mutex::new(vault),
            _lock: lock,
        })
    }

    /// Rewraps the DEK after a password change, preserving ciphertext payload.
    /// The caller must already own the unlocked vault, or unlock with the old
    /// password first. A server reset cannot recover this local key.
    ///
    /// # Errors
    /// Invalid new password, entropy, KDF or durable-write failure.
    pub fn rewrap(&self, password: &str) -> Result<(), Error> {
        let mut vault = self.vault.lock().map_err(|_| Error::Io)?;
        let previous = *vault.record();
        vault.rewrap(password)?;
        if let Err(error) = write_record(&self.path, vault.record()) {
            vault.record = previous;
            return Err(error);
        }
        Ok(())
    }
}

impl SecretStore for PasswordStore {
    fn home(&self) -> KeyHome {
        KeyHome::Password
    }

    fn load(&self, slot: &SecretSlot) -> Result<Option<String>, String> {
        if slot != &self.slot {
            return Err(Error::Scope.to_string());
        }
        let vault = self.vault.lock().map_err(|_| Error::Io.to_string())?;
        let bytes = vault.secrets().map_err(|e| e.to_string())?;
        let array = bytes
            .as_slice()
            .try_into()
            .map_err(|_| Error::Format.to_string())?;
        Ok(Some(
            encode_secrets(&baylee_seal::DeviceSecrets::from_bytes(array)).to_string(),
        ))
    }

    fn store(&self, slot: &SecretSlot, value: &str) -> Result<(), String> {
        if slot != &self.slot {
            return Err(Error::Scope.to_string());
        }
        let secrets = decode_secrets(value).ok_or_else(|| Error::Format.to_string())?;
        let mut vault = self.vault.lock().map_err(|_| Error::Io.to_string())?;
        let previous = *vault.record();
        vault
            .replace(&secrets.to_bytes())
            .map_err(|e| e.to_string())?;
        if let Err(error) = write_record(&self.path, vault.record()) {
            vault.record = previous;
            return Err(error.to_string());
        }
        Ok(())
    }

    fn forget(&self, slot: &SecretSlot) -> Result<(), String> {
        if slot != &self.slot {
            return Err(Error::Scope.to_string());
        }
        // Explicit device revocation archives this envelope. It must not be
        // silently registered again on the next launch, and remains recoverable.
        let archived = self
            .path
            .with_file_name(format!("revoked-{}.vault", uuid::Uuid::now_v7()));
        #[cfg(windows)]
        {
            super::windows::replace(&self.path, &archived).map_err(|e| e.to_string())
        }
        #[cfg(not(windows))]
        {
            std::fs::rename(&self.path, archived).map_err(|_| Error::Io.to_string())
        }
    }
}

fn options() -> OpenOptions {
    #[cfg(unix)]
    let mut options = OpenOptions::new();
    #[cfg(not(unix))]
    let options = OpenOptions::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    options
}

fn prepare_dir(dir: &Path) -> Result<(), Error> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::{DirBuilderExt as _, MetadataExt as _, PermissionsExt as _};
        // Walk all existing ancestors so neither a symlink nor a redirected
        // intermediate folder can silently select another account's file.
        for parent in dir.ancestors() {
            match std::fs::symlink_metadata(parent) {
                Ok(meta) if !meta.is_dir() || meta.file_type().is_symlink() => {
                    return Err(Error::Io);
                }
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(Error::Io),
                _ => {}
            }
        }
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)
            .map_err(|_| Error::Io)?;
        let meta = std::fs::metadata(dir).map_err(|_| Error::Io)?;
        if meta.uid() != rustix::process::geteuid().as_raw()
            || meta.permissions().mode() & 0o077 != 0
        {
            return Err(Error::Io);
        }
        Ok(())
    }
    #[cfg(windows)]
    {
        super::windows::create_dir(dir)
    }
    #[cfg(not(any(unix, windows)))]
    {
        // No private-file implementation for this unsupported native target.
        let _ = dir;
        Err(Error::Io)
    }
}

fn check_file(path: &Path) -> Result<bool, Error> {
    let meta = match std::fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(_) => return Err(Error::Io),
    };
    if !meta.is_file() || meta.file_type().is_symlink() {
        return Err(Error::Io);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
        let parent = path.parent().ok_or(Error::Io)?;
        let owner = std::fs::metadata(parent).map_err(|_| Error::Io)?.uid();
        if meta.uid() != owner
            || meta.uid() != rustix::process::geteuid().as_raw()
            || meta.nlink() != 1
            || meta.permissions().mode() & 0o077 != 0
        {
            return Err(Error::Io);
        }
    }
    #[cfg(windows)]
    super::windows::check(path)?;
    Ok(true)
}

/// A file of exactly `len` bytes; reads at most one byte more.
fn read_exact(path: &Path, len: usize) -> Result<Vec<u8>, Error> {
    let file = options().read(true).open(path).map_err(|_| Error::Io)?;
    let mut bytes = Vec::with_capacity(len + 1);
    file.take((len + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::Io)?;
    if bytes.len() != len {
        return Err(Error::Format);
    }
    Ok(bytes)
}

/// A side file's exact bytes, or none where it is absent, fails
/// `check_file`, cannot be read or has another length.
fn read_checked(path: &Path, len: usize) -> Option<Vec<u8>> {
    if !matches!(check_file(path), Ok(true)) {
        return None;
    }
    read_exact(path, len).ok()
}

fn write_record(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    let dir = path.parent().ok_or(Error::Io)?;
    let temp = dir.join(format!(".{}.tmp", uuid::Uuid::now_v7()));
    let result = (|| {
        let mut file = options()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|_| Error::Io)?;
        file.write_all(bytes).map_err(|_| Error::Io)?;
        file.sync_all().map_err(|_| Error::Io)?;
        drop(file);
        #[cfg(windows)]
        {
            super::windows::replace(&temp, path)
        }
        #[cfg(not(windows))]
        {
            std::fs::rename(&temp, path).map_err(|_| Error::Io)?;
            File::open(dir)
                .and_then(|f| f.sync_all())
                .map_err(|_| Error::Io)
        }
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}

fn lock_directory(root: &Path, scope: &Scope) -> Result<(PathBuf, File), Error> {
    // The configured root is trusted, and macOS commonly spells it through
    // /tmp or /var aliases. Resolve only that root; every vault component
    // below it is still checked and must never be a symlink.
    std::fs::create_dir_all(root).map_err(|_| Error::Io)?;
    let root = root.canonicalize().map_err(|_| Error::Io)?;
    let dir = root.join(scope.folder());
    for component in [root.join("vault"), dir.clone()] {
        if let Ok(meta) = std::fs::symlink_metadata(&component) {
            if meta.file_type().is_symlink() {
                return Err(Error::Io);
            }
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt as _;
                if meta.file_attributes() & 0x400 != 0 {
                    return Err(Error::Io);
                }
            }
        }
    }
    // The namespace parent must also be private: a writable parent would
    // let another OS user replace an otherwise private account directory.
    prepare_dir(&root.join("vault"))?;
    prepare_dir(&dir)?;
    let lock_path = dir.join("vault.lock");
    check_file(&lock_path)?;
    let lock = options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(|_| Error::Io)?;
    lock.try_lock().map_err(|_| Error::Busy)?;
    Ok((dir, lock))
}
