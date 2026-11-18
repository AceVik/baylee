//! Signs the synthetic administrator in, the way the game client does.
//!
//! `central` wrote the account's keys into the database and handed the
//! service its own; whether the two fit shows only when somebody signs in
//! against the running service. This is that sign-in: the client's own
//! (`baylee_account::Account::login`: discovery, the OPAQUE exchange, the
//! account document read and checked, the restore confirmed), then the
//! session revoked again. A stack that answers this has an account a person
//! can use, and one that does not says so before anybody types a password.
#![forbid(unsafe_code)]

use anyhow::{Result, anyhow};
use baylee_account::{Account, Endpoint};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    /// Central's native origin, a numeric loopback one.
    endpoint: String,
    username: String,
    password: String,
}

#[derive(Serialize)]
pub struct Reply {
    /// The gateways the account's document lists, as proof it was read.
    gateways: usize,
}

/// # Errors
/// Names the step that failed, and no value.
pub fn run(input: Input) -> Result<Reply> {
    let endpoint = Endpoint::development(&input.endpoint)
        .map_err(|_| anyhow!("the endpoint is not a loopback development origin"))?;
    let account = Account::login(endpoint, input.username, input.password, Vec::new())
        // The error is one of a handful of kinds and carries no value.
        .map_err(|error| anyhow!("signing in was refused ({error:?})"))?;
    let gateways = account.data().gateways.len();
    account
        .logout()
        .map_err(|error| anyhow!("signing out again failed ({error:?})"))?;
    Ok(Reply { gateways })
}
