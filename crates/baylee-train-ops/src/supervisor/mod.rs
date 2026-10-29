//! Literal, pinned local host supervision. Never reloads/restarts a child.
mod owned;
mod runtime;
#[cfg(test)]
mod tests;
use crate::files;
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    net::SocketAddr,
    path::{Path, PathBuf},
};
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Immutable native replacement for one reviewed `serve.py` profile.
pub struct Manifest {
    /// Must be one.
    pub version: u32,
    /// Existing profile root and working directory.
    pub root: PathBuf,
    /// Exact loopback listener; exclusive bind checked before spawn.
    pub bind: SocketAddr,
    /// Literal non-secret environment from the reviewed profile.
    pub environment: BTreeMap<String, String>,
    /// SHA256 seals for every executable, model artifact and runtime input.
    pub sha256: BTreeMap<PathBuf, String>,
    /// Ordered gateway/agent/seat/collection stages.
    pub stages: Vec<Stage>,
    /// Optional continuous health failure threshold; zero only checks child liveness.
    pub health_failure_limit: u32,
    /// Original host polling interval; stop requests remain responsive.
    pub poll_seconds: f64,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// One child process group, with explicit credential inheritance and readiness.
pub struct Stage {
    /// Stable log/status name.
    pub name: String,
    /// Literal argv. Shell expansion is never performed.
    pub command: Vec<String>,
    /// Pass the generated in-memory agent token to this child.
    pub agent_token: bool,
    /// `none`, `gateway` (`ok`) or `agent` (`agents.connected > 0`).
    pub readiness: String,
    /// Finite positive readiness timeout, at most 120 seconds.
    pub readiness_seconds: f64,
}
/// Verify a profile without starting a process or writing a runtime receipt.
/// # Errors
/// Refuses remote bind, missing executable seals, credentials or changed inputs.
pub fn check(path: &Path) -> Result<Manifest> {
    let manifest: Manifest = serde_json::from_value(files::read_json(path)?)?;
    ensure!(
        manifest.version == 1
            && manifest.root.is_absolute()
            && manifest.root.is_dir()
            && manifest.bind.ip().is_loopback()
            && manifest.bind.port() > 0,
        "invalid local supervisor manifest"
    );
    ensure!(
        !manifest.stages.is_empty() && manifest.stages.len() <= 4 && !manifest.sha256.is_empty(),
        "bounded pinned stages required"
    );
    ensure!(
        manifest.poll_seconds.is_finite()
            && manifest.poll_seconds > 0.0
            && manifest.poll_seconds <= 10.0,
        "invalid supervisor polling interval"
    );
    let mut names = BTreeSet::new();
    for stage in &manifest.stages {
        ensure!(
            !stage.agent_token
                || stage
                    .command
                    .first()
                    .and_then(|command| Path::new(command).file_name())
                    .is_some_and(|name| name == "baylee-agent" || name == "baylee-gateway"),
            "agent credentials only belong to gateway and agent binaries"
        );
        ensure!(
            !stage.name.is_empty()
                && stage
                    .name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
                && names.insert(&stage.name),
            "unsafe or duplicate stage name"
        );
        ensure!(
            !stage.command.is_empty()
                && Path::new(&stage.command[0]).is_absolute()
                && manifest.sha256.contains_key(Path::new(&stage.command[0])),
            "child executable must be absolute and sealed"
        );
        ensure!(
            ["none", "gateway", "agent"].contains(&stage.readiness.as_str())
                && stage.readiness_seconds.is_finite()
                && stage.readiness_seconds > 0.0
                && stage.readiness_seconds <= 120.0,
            "invalid readiness policy"
        );
    }
    ensure!(
        manifest
            .environment
            .iter()
            .all(|(key, value)| !key.contains("TOKEN")
                && !key.contains("SECRET")
                && !key.contains("PASSWORD")
                && !value.contains('\0')),
        "credentials cannot be stored in a supervisor manifest"
    );
    ensure!(
        manifest
            .environment
            .get("BAYLEE_EP")
            .is_some_and(|v| v == "cpu")
            && manifest
                .environment
                .get("CUDA_VISIBLE_DEVICES")
                .is_some_and(String::is_empty),
        "local supervisor must retain CPU-only profile"
    );
    for (input, digest) in &manifest.sha256 {
        ensure!(
            input.is_absolute() && files::sha256(input)? == *digest,
            "sealed supervisor input changed"
        );
    }
    let port = manifest.bind.port().to_string();
    ensure!(
        manifest.environment.get("PORT") == Some(&port)
            && manifest.environment.get("BAYLEE_BIND") == Some(&manifest.bind.ip().to_string()),
        "profile listener environment differs"
    );
    ensure!(
        manifest.environment.get("BAYLEE_GATEWAY") == Some(&format!("http://{}", manifest.bind)),
        "profile gateway endpoint differs"
    );
    let engine = manifest
        .environment
        .get("BAYLEE_ENGINE_BIN")
        .context("engine path required")?;
    ensure!(
        manifest.sha256.contains_key(Path::new(engine)),
        "engine executable must be sealed"
    );
    Ok(manifest)
}
pub use runtime::run;
