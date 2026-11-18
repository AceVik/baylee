//! The Tower authority for the local stack: a cohort published for the staged
//! Tower engine, and `baylee-tower-service` serving it
//! (`docs/tower-operations.md`).
#![forbid(unsafe_code)]

use super::{
    central,
    postgres::LocalPostgres,
    provision,
    services::{ManagedChild, staged_exe, wait_http_ready},
};
use anyhow::{Context, Result};
use serde_json::{Value, json};
use std::{
    path::Path,
    process::Command,
    time::{Duration, Instant},
};

/// The authority's loopback listener.
pub(super) const PORT: u16 = 28795;
pub(super) const DATABASE: &str = "baylee_tower";
const SCHEMA: &str = "tower_dev";
/// A verified game's record may be this large (`docs/tower-operations.md`).
const RECORD_BYTES: u64 = 64 * 1024 * 1024;

/// The authority's six credentials, made per start and kept in memory.
pub(super) struct Credentials {
    /// Central's award issuer: Central is provisioned with its digest.
    pub issuer: String,
    courier: String,
    /// The gateway's host credential (`BAYLEE_TOWER_HOST_TOKEN`).
    pub host: String,
    verifier: String,
    supervisor: String,
    publisher: String,
}

impl Credentials {
    pub(super) fn generate() -> Result<Self> {
        let token = || crate::secret::hex(32);
        Ok(Self {
            issuer: token()?,
            courier: token()?,
            host: token()?,
            verifier: token()?,
            supervisor: token()?,
            publisher: token()?,
        })
    }
}

/// Publishes a playable cohort for the staged Tower engine and starts the
/// authority. Central must be listening: the authority delivers awards to it.
pub(super) fn start(
    pg: &LocalPostgres,
    bin_dir: &Path,
    logs_dir: &Path,
    gateway_origin: &str,
    credentials: &Credentials,
) -> Result<ManagedChild> {
    let engine = staged_exe(bin_dir, "baylee-engine-tower");
    let started = Instant::now();
    let reply = provision::ask(
        bin_dir,
        logs_dir,
        "tower",
        &json!({
            "database_url": pg.url(DATABASE),
            "schema": SCHEMA,
            "engine": engine,
            "gateway": gateway_origin,
            "host": credentials.host,
            "verifier": credentials.verifier,
            "supervisor": credentials.supervisor,
            "publisher": credentials.publisher,
            "courier": credentials.courier,
        }),
    )?;
    println!(
        "Tower: cohort {} published ({} starters, {} stages) in {:.1} s",
        reply["cohort"].as_str().unwrap_or("?"),
        reply["starters"],
        reply["stages"],
        started.elapsed().as_secs_f32()
    );
    let config = service_config(&pg.url(DATABASE), &reply["cohort"], &engine, credentials);
    let mut service = ManagedChild::spawn_with_input(
        "baylee-tower-service",
        Command::new(staged_exe(bin_dir, "baylee-tower-service")).env("RUST_LOG", "info"),
        logs_dir,
        &serde_json::to_vec(&config).context("encoding the Tower authority's configuration")?,
    )?;
    wait_http_ready(
        PORT,
        "/health",
        "ready",
        &mut service,
        Duration::from_secs(30),
    )?;
    Ok(service)
}

/// The authority's stdin configuration: one worker verifying the one cohort.
fn service_config(
    database_url: &str,
    cohort: &Value,
    engine: &Path,
    credentials: &Credentials,
) -> Value {
    json!({
        "database_url": database_url,
        "schema": SCHEMA,
        "development": true,
        "service": {
            "listen": format!("127.0.0.1:{PORT}"),
            "origin": central::origin(PORT),
            "central": central::origin(central::NATIVE_PORT),
            "record_bytes": RECORD_BYTES,
            "workers": [{
                "cohort": cohort,
                "artifact": engine,
                "verifier": credentials.verifier,
                "supervisor": credentials.supervisor,
            }],
            "awards": {
                "courier": credentials.courier,
                "issuer": credentials.issuer,
            },
        },
    })
}

#[cfg(test)]
pub(super) fn config_for_test(cohort: &str) -> Value {
    let credentials = Credentials {
        issuer: "i".repeat(64),
        courier: "c".repeat(64),
        host: "h".repeat(64),
        verifier: "v".repeat(64),
        supervisor: "s".repeat(64),
        publisher: "p".repeat(64),
    };
    service_config(
        "postgres://postgres:secret@127.0.0.1:1/baylee_tower",
        &json!(cohort),
        Path::new("/stack/bin/baylee-engine-tower"),
        &credentials,
    )
}
