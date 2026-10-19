use std::sync::{Arc, LazyLock};

use keyring_core::CredentialStore;

use crate::Error;

static STORE: LazyLock<Result<Arc<CredentialStore>, Error>> = LazyLock::new(|| {
    #[cfg(windows)]
    let store = windows_native_keyring_store::Store::new();
    #[cfg(target_os = "macos")]
    let store = apple_native_keyring_store::keychain::Store::new();
    store
        .map(|store| store as Arc<CredentialStore>)
        .map_err(|_| Error::NoDefaultStore)
});

pub struct Entry(keyring_core::Entry);

impl Entry {
    pub fn new(service: &str, user: &str) -> Result<Self, Error> {
        STORE
            .as_ref()
            .map_err(|error| *error)?
            .build(service, user, None)
            .map(Self)
            .map_err(|error| redact(&error))
    }

    pub fn store_status() -> Result<(), Error> {
        STORE.as_ref().map(|_| ()).map_err(|error| *error)
    }

    pub fn get_credential(&self) -> Result<(), Error> {
        self.0
            .get_credential()
            .map(|_| ())
            .map_err(|error| redact(&error))
    }

    pub fn get_password(&self) -> Result<String, Error> {
        self.0.get_password().map_err(|error| redact(&error))
    }

    pub fn set_password(&self, password: &str) -> Result<(), Error> {
        self.0
            .set_password(password)
            .map_err(|error| redact(&error))
    }

    pub fn delete_credential(&self) -> Result<(), Error> {
        self.0.delete_credential().map_err(|error| redact(&error))
    }
}

fn redact(error: &keyring_core::Error) -> Error {
    match error {
        keyring_core::Error::NoEntry => Error::NoEntry,
        keyring_core::Error::NoDefaultStore => Error::NoDefaultStore,
        keyring_core::Error::Ambiguous(_) => Error::Ambiguous,
        _ => Error::Unavailable,
    }
}
