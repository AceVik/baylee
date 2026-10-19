use std::collections::HashMap;
use std::future::Future;
use std::time::Duration;

use futures_util::future::{Either, select};
use glib::prelude::StaticType as _;
use libsecret::prelude::{ItemExt, ServiceExt, ServiceExtManual};
use libsecret::{Item, SearchFlags, Service, ServiceFlags, Value};
use zeroize::Zeroizing;

use crate::Error;

const ENCRYPTED_SESSION: &str = "dh-ietf1024-sha256-aes128-cbc-pkcs7";

pub struct Entry {
    service: String,
    user: String,
}

impl Entry {
    pub fn new(service: &str, user: &str) -> Result<Self, Error> {
        // Keep validation local as well, for direct tests of this backend.
        crate::validate(service)?;
        crate::validate(user)?;
        Ok(Self {
            service: service.into(),
            user: user.into(),
        })
    }

    pub fn store_status() -> Result<(), Error> {
        run(async { open().await.map(|_| ()) })
    }

    fn attributes(&self) -> HashMap<&str, &str> {
        // Deliberately no schema-name or target filter: older keyring entries
        // use these exact two attributes and can live in any collection.
        HashMap::from([
            ("service", self.service.as_str()),
            ("username", self.user.as_str()),
        ])
    }

    async fn find(&self, service: &Service) -> Result<Item, Error> {
        let items = service
            .search_future(
                None,
                self.attributes(),
                SearchFlags::ALL | SearchFlags::UNLOCK,
            )
            .await
            .map_err(|_| Error::Unavailable)?;
        unique(items)
    }

    pub fn get_credential(&self) -> Result<(), Error> {
        run(async { self.find(&open().await?).await.map(|_| ()) })
    }

    pub fn get_password(&self) -> Result<String, Error> {
        run(async {
            let service = open().await?;
            let item = self.find(&service).await?;
            item.load_secret_future()
                .await
                .map_err(|_| Error::Unavailable)?;
            let secret = item.secret().ok_or(Error::Unavailable)?;
            let bytes = Zeroizing::new(secret.get());
            std::str::from_utf8(&bytes)
                .map(str::to_owned)
                .map_err(|_| Error::Unavailable)
        })
    }

    pub fn set_password(&self, password: &str) -> Result<(), Error> {
        run(async {
            let service = open().await?;
            let secret = Value::new(password, "text/plain; charset=utf-8");
            match self.find(&service).await {
                Ok(item) => item.set_secret_future(&secret).await,
                Err(Error::NoEntry) => {
                    service
                        .store_future(
                            None,
                            self.attributes(),
                            Some(libsecret::COLLECTION_DEFAULT.as_str()),
                            &format!("keyring:{}@{}", self.user, self.service),
                            &secret,
                        )
                        .await
                }
                Err(error) => return Err(error),
            }
            .map_err(|_| Error::Unavailable)
        })
    }

    pub fn delete_credential(&self) -> Result<(), Error> {
        run(async {
            self.find(&open().await?)
                .await?
                .delete_future()
                .await
                .map_err(|_| Error::Unavailable)
        })
    }
}

async fn open() -> Result<Service, Error> {
    // Use this operation's own service object, not libsecret's process-global
    // cache. Check the actual negotiated algorithm before any secret transfer.
    let service = Service::open_future(Service::static_type(), None, ServiceFlags::OPEN_SESSION)
        .await
        .map_err(|_| Error::NoDefaultStore)?;
    encrypted(service.session_algorithms().as_deref())?;
    Ok(service)
}

fn encrypted(algorithm: Option<&str>) -> Result<(), Error> {
    if algorithm == Some(ENCRYPTED_SESSION) {
        Ok(())
    } else {
        Err(Error::InsecureSession)
    }
}

fn unique<T>(mut items: Vec<T>) -> Result<T, Error> {
    match items.len() {
        0 => Err(Error::NoEntry),
        1 => items.pop().ok_or(Error::NoEntry),
        _ => Err(Error::Ambiguous),
    }
}

fn run<T>(operation: impl Future<Output = Result<T, Error>>) -> Result<T, Error> {
    run_with_timeout(operation, Duration::from_secs(30))
}

fn run_with_timeout<T>(
    operation: impl Future<Output = Result<T, Error>>,
    timeout: Duration,
) -> Result<T, Error> {
    let context = glib::MainContext::new();
    context
        .with_thread_default(|| {
            context.block_on(async {
                match select(Box::pin(operation), Box::pin(glib::timeout_future(timeout))).await {
                    Either::Left((result, _)) => result,
                    // Dropping GioFuture cancels the outstanding local request.
                    // A timed-out write has an indeterminate provider outcome.
                    Either::Right(((), _)) => Err(Error::Timeout),
                }
            })
        })
        .map_err(|_| Error::Unavailable)?
}

#[cfg(test)]
#[path = "linux/tests.rs"]
mod tests;
