//! Versioned password vault. Argon2id v1.3, 64 MiB, three passes, four
//! lanes (RFC 9106's second recommended profile), a random 16-byte salt and
//! XChaCha20-Poly1305 wrap a random 256-bit data key. A separate random
//! nonce seals the device secrets under that data key. Version 1 fixes all
//! lengths and KDF parameters: disk input cannot request unbounded work.
//!
//! Neither password reset nor decryption failure creates a replacement.
//! Rewrapping preserves the data key and payload; old backups still require
//! their old password. This protects offline files, not a compromised process.

use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

mod account;
#[cfg(not(target_arch = "wasm32"))]
mod file;
pub use account::{MISS_FILE, MISS_LEN, WRAP_FILE, WRAP_LEN, is_miss_marker, miss_marker};
#[cfg(windows)]
mod windows;
#[cfg(not(target_arch = "wasm32"))]
pub use file::{ContextGuard, PasswordStore, attach_account_keys};

const MAGIC: &[u8; 8] = b"BAYVAU01";
const HEADER: usize = 24;
const WRAP_END: usize = HEADER + 24 + 48;
/// Exact version-one size; no variable-size allocation from disk input.
pub const RECORD_LEN: usize = WRAP_END + 24 + 112;
/// Limit before any KDF work. Passwords are UTF-8 bytes, without normalization.
pub const MAX_PASSWORD: usize = 1024;
/// The ciphertext file, separate from legacy keys and chat history.
pub const FILE_NAME: &str = "password.vault";

/// A vault failure, with no secret values or OS error messages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// Wrong password, changed context, or damaged ciphertext.
    Authentication,
    /// Unknown version, invalid length, or invalid input.
    Format,
    /// No safe operating-system randomness is available.
    Entropy,
    /// The file cannot be read or durably written.
    Io,
    /// Another process owns this account's vault.
    Busy,
    /// The vault already exists; creation must never overwrite it.
    Exists,
    /// No vault exists; creation or legacy import requires an explicit choice.
    Missing,
    /// The account or gateway does not match the unlocked store.
    Scope,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Authentication => "The vault could not be unlocked. Check the original vault password; no data was changed.",
            Self::Format => "The vault or password has an unsupported format; no data was changed.",
            Self::Entropy => "Secure randomness is unavailable.",
            Self::Io => "The vault file could not be safely read or saved.",
            Self::Busy => "This vault is open in another Baylee process.",
            Self::Exists => "A vault already exists. Unlock it instead.",
            Self::Missing => "Choose to import existing keys or create a new encrypted vault.",
            Self::Scope => "The vault belongs to another account or gateway.",
        })
    }
}
impl std::error::Error for Error {}

/// Canonical, collision-resistant account/gateway binding, also used as AAD.
/// The full URL is retained (including a case-sensitive path), not just host.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Scope([u8; 32]);

impl Scope {
    /// Binds a validated account tag to an HTTP(S) gateway base URL. The
    /// live gateway binds by the address its keys are bound to
    /// (`gateway_list::keyed`), so its vault outlives a move of its address.
    ///
    /// # Errors
    /// [`Error::Scope`] for malformed or ambiguous inputs.
    pub fn new(tag: &str, gateway: &str) -> Result<Self, Error> {
        let gateway = crate::lobby::gateway_list::keyed(gateway.trim());
        if tag.is_empty()
            || tag.len() > 128
            || !tag.bytes().all(|b| b.is_ascii_alphanumeric())
            || gateway.len() > 2048
        {
            return Err(Error::Scope);
        }
        let mut url = url::Url::parse(gateway.trim()).map_err(|_| Error::Scope)?;
        if !matches!(url.scheme(), "http" | "https")
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err(Error::Scope);
        }
        let path = url.path().trim_end_matches('/').to_string();
        url.set_path(&path);
        let mut hash = Sha256::new();
        hash.update(b"baylee/password-vault/scope/v1\0");
        hash.update(tag.as_bytes());
        hash.update([0]);
        hash.update(url.as_str().trim_end_matches('/').as_bytes());
        Ok(Self(hash.finalize().into()))
    }

    /// The scope a vault folder's name says (its last component, 64
    /// lowercase hex digits), or `None` for any other name.
    #[must_use]
    pub fn from_folder_name(name: &str) -> Option<Self> {
        if name.len() != 64 || !name.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
            return None;
        }
        let mut bytes = [0; 32];
        for (i, byte) in bytes.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&name[2 * i..2 * i + 2], 16).ok()?;
        }
        Some(Self(bytes))
    }

    /// The 32 bytes a key is derived for (`Vault::account_wrap`).
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Relative account folder for encrypted keys. No user-controlled path parts.
    #[must_use]
    pub fn folder(&self) -> String {
        let hex = crate::social::sealing::hex(&self.0);
        format!("vault/{hex}")
    }
}

/// An unlocked envelope. Debug deliberately omits both key and ciphertext.
pub struct Vault {
    scope: Scope,
    key: Zeroizing<[u8; 32]>,
    record: [u8; RECORD_LEN],
}

impl std::fmt::Debug for Vault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Vault([redacted])")
    }
}

impl Vault {
    /// Creates an encrypted envelope using OS entropy. Password is never retained.
    ///
    /// # Errors
    /// Invalid password length, unavailable entropy, or KDF/cipher failure.
    pub fn create(scope: Scope, password: &str, secrets: &[u8; 96]) -> Result<Self, Error> {
        let mut key = Zeroizing::new([0; 32]);
        random(&mut key[..])?;
        let mut vault = Self {
            scope,
            key,
            record: [0; RECORD_LEN],
        };
        vault.rewrap(password)?;
        vault.replace(secrets)?;
        Ok(vault)
    }

    /// Authenticates the full envelope before releasing an unlocked key.
    ///
    /// # Errors
    /// Invalid size/version, password, scope, or damaged ciphertext.
    pub fn unlock(scope: Scope, password: &str, bytes: &[u8]) -> Result<Self, Error> {
        let record: [u8; RECORD_LEN] = bytes.try_into().map_err(|_| Error::Format)?;
        if &record[..8] != MAGIC {
            return Err(Error::Format);
        }
        let wrapping = derive(password, &record[8..HEADER])?;
        let plain = open(
            &wrapping,
            &record[HEADER..WRAP_END],
            &aad(&scope, b"wrap", &record[..HEADER]),
        )?;
        let mut key = Zeroizing::new([0; 32]);
        if plain.len() != 32 {
            return Err(Error::Format);
        }
        key.copy_from_slice(&plain);
        Self::with_key(scope, key, &record)
    }

    /// A version-one record opened with its data key however that was
    /// unwrapped; the payload is authenticated before the vault is answered.
    fn with_key(scope: Scope, key: Zeroizing<[u8; 32]>, record: &[u8]) -> Result<Self, Error> {
        let record: [u8; RECORD_LEN] = record.try_into().map_err(|_| Error::Format)?;
        if &record[..8] != MAGIC {
            return Err(Error::Format);
        }
        let vault = Self { scope, key, record };
        vault.secrets()?;
        Ok(vault)
    }

    /// Authenticated device secrets, zeroed on drop.
    ///
    /// # Errors
    /// Authentication failure; no empty/default key is substituted.
    pub fn secrets(&self) -> Result<Zeroizing<Vec<u8>>, Error> {
        open(
            &self.key,
            &self.record[WRAP_END..],
            &aad(&self.scope, b"device", MAGIC),
        )
    }

    /// Rotates the device payload while keeping its random encryption key.
    ///
    /// # Errors
    /// Entropy or cipher failure. The old record remains intact on failure.
    pub fn replace(&mut self, secrets: &[u8; 96]) -> Result<(), Error> {
        let encrypted = seal(&self.key, secrets, &aad(&self.scope, b"device", MAGIC))?;
        self.record[WRAP_END..].copy_from_slice(&encrypted);
        Ok(())
    }

    /// Changes only the password wrapper, with fresh salt and nonce.
    ///
    /// # Errors
    /// Invalid password, entropy or KDF failure. The old wrapper remains intact.
    pub fn rewrap(&mut self, password: &str) -> Result<(), Error> {
        let mut header = [0; HEADER];
        header[..8].copy_from_slice(MAGIC);
        random(&mut header[8..])?;
        let key = derive(password, &header[8..])?;
        let encrypted = seal(&key, &self.key[..], &aad(&self.scope, b"wrap", &header))?;
        self.record[..HEADER].copy_from_slice(&header);
        self.record[HEADER..WRAP_END].copy_from_slice(&encrypted);
        Ok(())
    }

    /// Serialized ciphertext only.
    #[must_use]
    pub const fn record(&self) -> &[u8; RECORD_LEN] {
        &self.record
    }
}

fn random(out: &mut [u8]) -> Result<(), Error> {
    getrandom::fill(out).map_err(|_| Error::Entropy)
}

fn derive(password: &str, salt: &[u8]) -> Result<Zeroizing<[u8; 32]>, Error> {
    if password.is_empty() || password.len() > MAX_PASSWORD {
        return Err(Error::Format);
    }
    let params = Params::new(65_536, 3, 4, Some(32)).map_err(|_| Error::Format)?;
    let kdf = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = Zeroizing::new([0; 32]);
    kdf.hash_password_into(password.as_bytes(), salt, &mut key[..])
        .map_err(|_| Error::Format)?;
    Ok(key)
}

fn aad(scope: &Scope, purpose: &[u8], header: &[u8]) -> Vec<u8> {
    [
        b"baylee/password-vault/v1\0".as_slice(),
        &scope.0,
        purpose,
        header,
    ]
    .concat()
}

fn seal(key: &[u8; 32], plain: &[u8], aad: &[u8]) -> Result<Vec<u8>, Error> {
    let mut nonce = [0; 24];
    random(&mut nonce)?;
    let cipher = XChaCha20Poly1305::new(key.into());
    let ct = cipher
        .encrypt(&XNonce::from(nonce), Payload { msg: plain, aad })
        .map_err(|_| Error::Format)?;
    Ok([nonce.as_slice(), &ct].concat())
}

fn open(key: &[u8; 32], record: &[u8], aad: &[u8]) -> Result<Zeroizing<Vec<u8>>, Error> {
    let nonce: [u8; 24] = record
        .get(..24)
        .ok_or(Error::Format)?
        .try_into()
        .map_err(|_| Error::Format)?;
    XChaCha20Poly1305::new(key.into())
        .decrypt(
            &XNonce::from(nonce),
            Payload {
                msg: &record[24..],
                aad,
            },
        )
        .map(Zeroizing::new)
        .map_err(|_| Error::Authentication)
}

#[cfg(test)]
mod tests;
