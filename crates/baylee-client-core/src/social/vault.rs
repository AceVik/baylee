//! Legacy secret-store adapters and the password-encrypted local vault.
//! Normal client startup uses [`password`], never the OS credential store.

use std::collections::BTreeMap;
use std::sync::Mutex;

use baylee_seal::DeviceSecrets;
use baylee_seal::b64::url;
use zeroize::Zeroizing;

pub mod password;

/// The credential store's service name for chat keys.
pub const SERVICE: &str = "baylee-chat";

/// The fallback file's name, in the account's chat folder.
pub const KEY_FILE: &str = "device.key";

/// One device's entry in a store.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SecretSlot {
    /// Always [`SERVICE`].
    pub service: &'static str,
    /// `{tag}@{host[:port]}`.
    pub account: String,
    /// Full context for modern stores; `account` retains the legacy OS entry.
    binding: Option<password::Scope>,
}

impl SecretSlot {
    /// The entry for the account `tag` on the gateway at `gateway` (a base
    /// URL; its scheme and path are not part of the entry). The live gateway
    /// is named by the address its keys are bound to (`gateway_list::keyed`).
    #[must_use]
    pub fn new(tag: &str, gateway: &str) -> Self {
        let gateway = crate::lobby::gateway_list::keyed(gateway.trim());
        Self {
            service: SERVICE,
            account: format!("{tag}@{}", host_of(gateway)),
            binding: password::Scope::new(tag, gateway).ok(),
        }
    }
}

/// `host[:port]` of a base URL, lower case.
fn host_of(gateway: &str) -> String {
    let rest = gateway
        .trim()
        .split_once("://")
        .map_or(gateway.trim(), |(_, rest)| rest);
    rest.split(['/', '?', '#'])
        .next()
        .unwrap_or_default()
        .rsplit('@')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase()
}

/// Where the keys are kept, which the notice says (§11.8).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KeyHome {
    /// Not known yet (the store was not opened).
    #[default]
    Unknown,
    /// The operating system's credential store.
    Keychain,
    /// Authenticated ciphertext protected by the account password.
    Password,
    /// Client-encrypted central account archive; each installation has its own active identity.
    Central,
    /// A file only the user's account can read: no store opened.
    File,
    /// Memory only (tests, or a platform with neither).
    Memory,
}

/// A place secrets are kept. Implementations never log a value.
pub trait SecretStore: Send + Sync {
    /// Where this store keeps them.
    fn home(&self) -> KeyHome;

    /// The value in `slot`, if any.
    ///
    /// # Errors
    ///
    /// A sentence when the store can't be read; never a secret.
    fn load(&self, slot: &SecretSlot) -> Result<Option<String>, String>;

    /// Keeps `value` in `slot`, replacing what was there.
    ///
    /// # Errors
    ///
    /// A sentence when it can't be written.
    fn store(&self, slot: &SecretSlot, value: &str) -> Result<(), String>;

    /// Removes `slot`; nothing there is not an error.
    ///
    /// # Errors
    ///
    /// A sentence when it can't be removed.
    fn forget(&self, slot: &SecretSlot) -> Result<(), String>;
}

/// The secrets as a store keeps them: base64url of the 96 bytes.
#[must_use]
pub fn encode_secrets(secrets: &DeviceSecrets) -> Zeroizing<String> {
    Zeroizing::new(url::encode(&secrets.to_bytes()[..]))
}

/// The secrets from a store's value; `None` for anything else.
#[must_use]
pub fn decode_secrets(value: &str) -> Option<DeviceSecrets> {
    let value = value.trim();
    if value.len() != 128 {
        return None;
    }
    let bytes = Zeroizing::new(url::decode(value).ok()?);
    let array: &[u8; baylee_seal::SECRETS_LEN] = bytes.as_slice().try_into().ok()?;
    Some(DeviceSecrets::from_bytes(array))
}

/// A store in memory, for tests and for C's fake transport.
#[derive(Default)]
pub struct MemorySecrets {
    values: Mutex<BTreeMap<SecretSlot, Zeroizing<String>>>,
}

impl MemorySecrets {
    /// An empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl SecretStore for MemorySecrets {
    fn home(&self) -> KeyHome {
        KeyHome::Memory
    }

    fn load(&self, slot: &SecretSlot) -> Result<Option<String>, String> {
        let values = self
            .values
            .lock()
            .map_err(|_| "the store is poisoned".to_string())?;
        Ok(values.get(slot).map(|v| v.to_string()))
    }

    fn store(&self, slot: &SecretSlot, value: &str) -> Result<(), String> {
        let mut values = self
            .values
            .lock()
            .map_err(|_| "the store is poisoned".to_string())?;
        values.insert(slot.clone(), Zeroizing::new(value.to_string()));
        Ok(())
    }

    fn forget(&self, slot: &SecretSlot) -> Result<(), String> {
        let mut values = self
            .values
            .lock()
            .map_err(|_| "the store is poisoned".to_string())?;
        values.remove(slot);
        Ok(())
    }
}

/// The fallback: one file, `0600`, in a `0700` folder, written through a
/// temp file and renamed. A file or folder another user may read, or a file
/// owned by another user, is refused rather than read. Without unix modes
/// (Windows, where the Credential Manager is always there) it refuses.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug)]
pub struct FileSecrets {
    // The legacy plaintext fallback was Unix-only; retained solely for import.
    #[cfg_attr(not(unix), allow(dead_code))]
    path: std::path::PathBuf,
}

#[cfg(not(target_arch = "wasm32"))]
impl FileSecrets {
    /// The store at `path` (`<config>/chat/<gw>/<tag>/device.key`).
    #[must_use]
    pub fn new(path: impl Into<std::path::PathBuf>) -> Self {
        Self { path: path.into() }
    }

    #[cfg(unix)]
    fn dir(&self) -> Result<&std::path::Path, String> {
        use std::os::unix::fs::{DirBuilderExt as _, PermissionsExt as _};
        let dir = self
            .path
            .parent()
            .ok_or_else(|| "the key file has no folder".to_string())?;
        if !dir.exists() {
            std::fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(dir)
                .map_err(|e| format!("the key folder could not be made: {}", e.kind()))?;
        }
        let meta = std::fs::metadata(dir)
            .map_err(|e| format!("the key folder could not be read: {}", e.kind()))?;
        if !meta.is_dir() || meta.permissions().mode() & 0o077 != 0 {
            return Err("the key folder may be read by other users; it must be 0700".into());
        }
        Ok(dir)
    }

    /// Whether the file may be read: there, a plain file, this folder's
    /// owner's, readable by nobody else.
    #[cfg(unix)]
    fn trusted(&self) -> Result<bool, String> {
        use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
        let dir = std::fs::metadata(self.dir()?)
            .map_err(|e| format!("the key folder could not be read: {}", e.kind()))?;
        let meta = match std::fs::symlink_metadata(&self.path) {
            Ok(meta) => meta,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(e) => return Err(format!("the key file could not be read: {}", e.kind())),
        };
        if !meta.is_file() || meta.uid() != dir.uid() || meta.permissions().mode() & 0o077 != 0 {
            return Err("the key file is not this account's own; it was not read".into());
        }
        Ok(true)
    }
}

#[cfg(all(not(target_arch = "wasm32"), unix))]
impl SecretStore for FileSecrets {
    fn home(&self) -> KeyHome {
        KeyHome::File
    }

    fn load(&self, _slot: &SecretSlot) -> Result<Option<String>, String> {
        use std::io::Read as _;
        if !self.trusted()? {
            return Ok(None);
        }
        let mut text = Zeroizing::new(String::new());
        std::fs::File::open(&self.path)
            .and_then(|file| file.take(4097).read_to_string(&mut text))
            .map_err(|e| format!("the key file could not be read: {}", e.kind()))?;
        if text.len() > 4096 {
            return Err("the legacy key file is too large".into());
        }
        let value = text.trim();
        Ok((!value.is_empty()).then(|| value.to_string()))
    }

    fn store(&self, _slot: &SecretSlot, value: &str) -> Result<(), String> {
        use std::io::Write as _;
        use std::os::unix::fs::OpenOptionsExt as _;
        let dir = self.dir()?;
        // A file another user put there is not replaced blindly.
        self.trusted()?;
        let temp = dir.join(format!(".{KEY_FILE}.{}.tmp", std::process::id()));
        let _ = std::fs::remove_file(&temp);
        let written = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temp)
            .and_then(|mut file| {
                file.write_all(value.as_bytes())?;
                file.sync_all()
            })
            .and_then(|()| std::fs::rename(&temp, &self.path));
        if let Err(e) = written {
            let _ = std::fs::remove_file(&temp);
            return Err(format!("the key could not be written: {}", e.kind()));
        }
        Ok(())
    }

    fn forget(&self, _slot: &SecretSlot) -> Result<(), String> {
        match std::fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(format!("the key could not be removed: {}", e.kind())),
        }
    }
}

#[cfg(all(not(target_arch = "wasm32"), not(unix)))]
impl SecretStore for FileSecrets {
    fn home(&self) -> KeyHome {
        KeyHome::File
    }

    fn load(&self, _slot: &SecretSlot) -> Result<Option<String>, String> {
        Err("a key file needs unix file modes".into())
    }

    fn store(&self, _slot: &SecretSlot, _value: &str) -> Result<(), String> {
        Err("a key file needs unix file modes".into())
    }

    fn forget(&self, _slot: &SecretSlot) -> Result<(), String> {
        Ok(())
    }
}

/// The operating system's credential store. Never opened by a test.
#[cfg(any(windows, target_os = "macos", target_os = "linux"))]
#[derive(Clone, Copy, Debug)]
pub struct OsSecrets;

#[cfg(any(windows, target_os = "macos", target_os = "linux"))]
impl OsSecrets {
    /// Whether this machine's store opens at all.
    #[must_use]
    pub fn available() -> bool {
        keyring::Entry::store_status().is_ok()
    }

    fn entry(slot: &SecretSlot) -> Result<keyring::Entry, String> {
        keyring::Entry::new(slot.service, &slot.account).map_err(said)
    }
}

/// A store's error as a sentence: its messages name the entry at most,
/// never a secret.
#[cfg(any(windows, target_os = "macos", target_os = "linux"))]
fn said(error: keyring::Error) -> String {
    match error {
        keyring::Error::NoDefaultStore => {
            "this machine's credential store could not be opened".into()
        }
        other => format!("the credential store refused: {other}")
            .chars()
            .take(200)
            .collect(),
    }
}

#[cfg(any(windows, target_os = "macos", target_os = "linux"))]
impl SecretStore for OsSecrets {
    fn home(&self) -> KeyHome {
        KeyHome::Keychain
    }

    fn load(&self, slot: &SecretSlot) -> Result<Option<String>, String> {
        match Self::entry(slot)?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(said(e)),
        }
    }

    fn store(&self, slot: &SecretSlot, value: &str) -> Result<(), String> {
        Self::entry(slot)?.set_password(value).map_err(said)
    }

    fn forget(&self, slot: &SecretSlot) -> Result<(), String> {
        match Self::entry(slot)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(said(e)),
        }
    }
}
