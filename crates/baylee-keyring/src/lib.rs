//! Native credential storage shared by the client and its language-model seat.
//!
//! Windows/macOS retain their existing native providers and entry naming.
//! Linux uses GNOME libsecret with an encrypted Secret Service session and the
//! original `service`/`username` attributes. Calls block; use a worker thread.
//! No operation falls back to a file or silently replaces an ambiguous entry.

#![forbid(unsafe_code)]

#[cfg(target_os = "linux")]
#[path = "linux.rs"]
mod platform;
#[cfg(any(windows, target_os = "macos"))]
#[path = "native.rs"]
mod platform;
#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
#[path = "unsupported.rs"]
mod platform;

/// Redacted failures: provider errors and secret bytes never enter diagnostics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// No matching credential exists.
    NoEntry,
    /// This platform has no usable credential store.
    NoDefaultStore,
    /// More than one credential matches the original lookup attributes.
    Ambiguous,
    /// The provider refused an operation or returned malformed secret data.
    Unavailable,
    /// An identifier or value cannot be represented by the platform API.
    Invalid,
    /// The provider negotiated an unencrypted or unsupported session.
    InsecureSession,
    /// The complete operation exceeded its deadline.
    Timeout,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NoEntry => "no matching credential exists",
            Self::NoDefaultStore => "no native credential store is available",
            Self::Ambiguous => "multiple credentials match this entry",
            Self::Unavailable => "the credential store refused the operation",
            Self::Invalid => "invalid credential-store input",
            Self::InsecureSession => "the credential store did not establish an encrypted session",
            Self::Timeout => "the credential-store operation timed out",
        })
    }
}

impl std::error::Error for Error {}

/// A credential's address, never its secret.
pub struct Entry(platform::Entry);

impl Entry {
    /// Select an entry using the same identifiers as previous Baylee releases.
    ///
    /// # Errors
    /// Returns an error for invalid identifiers or an unavailable provider.
    pub fn new(service: &str, user: &str) -> Result<Self, Error> {
        validate(service)?;
        validate(user)?;
        platform::Entry::new(service, user).map(Self)
    }

    /// Check availability without reading or writing any credential.
    ///
    /// # Errors
    /// Returns an error if the provider cannot be initialized securely.
    pub fn store_status() -> Result<(), Error> {
        platform::Entry::store_status()
    }

    /// Check existence without retrieving the secret.
    ///
    /// # Errors
    /// Returns [`Error::NoEntry`] when absent, or a provider failure.
    pub fn get_credential(&self) -> Result<(), Error> {
        self.0.get_credential()
    }

    /// Read the secret. The caller owns and must protect the returned string.
    ///
    /// # Errors
    /// Returns an error for missing, ambiguous, malformed or inaccessible data.
    pub fn get_password(&self) -> Result<String, Error> {
        self.0.get_password()
    }

    /// Create or update one unambiguous credential.
    ///
    /// # Errors
    /// Returns an error for invalid data, ambiguity or provider failure.
    pub fn set_password(&self, password: &str) -> Result<(), Error> {
        if password.len() > 1024 * 1024 || password.contains('\0') {
            return Err(Error::Invalid);
        }
        self.0.set_password(password)
    }

    /// Remove one unambiguous credential.
    ///
    /// # Errors
    /// Returns [`Error::NoEntry`] when absent, or a provider failure.
    pub fn delete_credential(&self) -> Result<(), Error> {
        self.0.delete_credential()
    }
}

fn validate(value: &str) -> Result<(), Error> {
    if value.is_empty() || value.len() > 4096 || value.contains('\0') {
        Err(Error::Invalid)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_addresses_before_opening_a_store() {
        for invalid in ["", "hidden\0suffix", &"x".repeat(4097)] {
            assert!(matches!(Entry::new(invalid, "user"), Err(Error::Invalid)));
            assert!(matches!(
                Entry::new("service", invalid),
                Err(Error::Invalid)
            ));
        }
    }
}
