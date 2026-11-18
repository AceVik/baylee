//! Signs the synthetic administrator in, the way the game client does.
//!
//! `central` wrote the account's keys into the database and handed the
//! service its own; whether the two fit shows only when somebody signs in
//! against the running service. This is that sign-in: the client's own
//! (`baylee_account::Account::login`: discovery, the OPAQUE exchange, the
//! account document read and checked, the restore confirmed), then the
//! session revoked again. A stack that answers this has an account a person
//! can use, and one that does not says so before anybody types a password.
//!
//! With a gateway named, the account is also admitted there (Central's token
//! for that gateway, which the gateway checks against the keys its bridge
//! gave it) and asks it for its Tower capacity (`GET /tower`): the gateway
//! answers that only after the Tower authority accepted it as a host, with
//! the Tower engines its agents have free. That is the whole chain a Tower
//! duel needs, short of starting one.
#![forbid(unsafe_code)]

use anyhow::{Context as _, Result, anyhow, bail};
use baylee_account::{Account, Endpoint};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    /// Central's native origin, a numeric loopback one.
    endpoint: String,
    username: String,
    password: String,
    /// The stack's gateway, to be admitted at and asked for its Tower
    /// capacity; absent while no gateway runs yet.
    #[serde(default)]
    gateway: Option<String>,
}

#[derive(Serialize)]
pub struct Reply {
    /// The gateways the account's document lists, as proof it was read.
    gateways: usize,
    /// What the gateway said of its Tower hosting, when one was named.
    tower: Option<Hosting>,
}

/// The gateway's `GET /tower` answer, reduced to what the stack compares.
#[derive(Serialize)]
pub struct Hosting {
    /// The Tower authority the gateway hosts for.
    origin: String,
    /// Tower engines its agents have free.
    engines: usize,
}

/// `GET /tower` as the gateway writes it (`docs/protocol.md`).
#[derive(Deserialize)]
struct Availability {
    origin: String,
    engines: Vec<serde_json::Value>,
}

/// How long the gateway may still be waiting for its agent's inventory.
const PATIENCE: Duration = Duration::from_secs(20);

/// # Errors
/// Names the step that failed, and no value.
pub fn run(input: Input) -> Result<Reply> {
    let endpoint = Endpoint::development(&input.endpoint)
        .map_err(|_| anyhow!("the endpoint is not a loopback development origin"))?;
    if let Some(gateway) = &input.gateway {
        central_api::gateway::validate_origin(gateway, true)
            .map_err(|_| anyhow!("the gateway is not a loopback development origin"))?;
    }
    let mut account = Account::login(endpoint.clone(), input.username, input.password, Vec::new())
        // The error is one of a handful of kinds and carries no value.
        .map_err(|error| anyhow!("signing in was refused ({error:?})"))?;
    let gateways = account.data().gateways.len();
    let tower = input
        .gateway
        .as_deref()
        .map(|gateway| hosting(&mut account, &endpoint, gateway));
    // The session ends whatever the gateway said.
    let ended = account.logout();
    let tower = tower.transpose()?;
    ended.map_err(|error| anyhow!("signing out again failed ({error:?})"))?;
    Ok(Reply { gateways, tower })
}

/// Accepts the terms in force, Central's and the gateway's, the way the
/// client does after showing them. The account is the stack's synthetic one,
/// on a Central that lives as long as the stack: it would otherwise meet both
/// sheets again at every start, and a gateway admits nobody before them.
fn accept_terms(account: &mut Account, endpoint: &Endpoint, gateway: &str) -> Result<()> {
    use baylee_account::terms::TermsSubject;
    let refused = |step: &'static str| {
        move |error: baylee_account::Error| anyhow!("{step} was refused ({error:?})")
    };
    if !account
        .central_terms_status()
        .map_err(refused("reading Central's terms"))?
        .accepted
        && let Some(terms) = account
            .central_terms()
            .map_err(refused("reading Central's terms"))?
    {
        account
            .accept_terms(TermsSubject::Central, &terms.version)
            .map_err(refused("accepting Central's terms"))?;
    }
    let http = baylee_account::CentralHttp::new(endpoint.clone());
    if let Some(terms) = baylee_account::sdk::gateway_terms(&http, gateway)
        .map_err(refused("reading the gateway's terms"))?
    {
        account
            .accept_terms(
                TermsSubject::Gateway {
                    origin: gateway.to_owned(),
                },
                &terms.version,
            )
            .map_err(refused("accepting the gateway's terms"))?;
    }
    Ok(())
}

fn hosting(account: &mut Account, endpoint: &Endpoint, gateway: &str) -> Result<Hosting> {
    accept_terms(account, endpoint, gateway)?;
    let admission = account
        .admit(gateway)
        .map_err(|error| anyhow!("the gateway did not admit the account ({error:?})"))?;
    let deadline = Instant::now() + PATIENCE;
    loop {
        let answer = ureq::get(format!("{gateway}/tower"))
            .header("Authorization", format!("Bearer {}", admission.token))
            .call();
        match answer {
            Ok(response) => {
                let found: Availability = response
                    .into_body()
                    .read_json()
                    .context("the gateway's Tower answer is not the expected JSON")?;
                return Ok(Hosting {
                    origin: found.origin,
                    engines: found.engines.len(),
                });
            }
            // No compatible agent yet, or the authority has not answered the
            // gateway: both pass within seconds of a start.
            Err(ureq::Error::StatusCode(503 | 429)) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(500));
            }
            Err(ureq::Error::StatusCode(status)) => {
                bail!("the gateway refused its Tower capacity with status {status}")
            }
            Err(_) => bail!("the gateway did not answer for its Tower capacity"),
        }
    }
}
