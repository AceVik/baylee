//! One request to `baylee-dev-provision`, the helper that links what `cargo
//! dev` must not: Central's server crates and the Tower store.
//!
//! The request and the reply are JSON over the helper's stdin and stdout, and
//! both carry secrets, so neither is printed or kept; only the helper's
//! stderr, which names a failed step and no value, goes to a log.
#![forbid(unsafe_code)]

use super::services::staged_exe;
use anyhow::{Context, Result, ensure};
use serde_json::Value;
use std::{
    fs::File,
    io::Write as _,
    path::Path,
    process::{Command, Stdio},
};

pub(super) fn ask(bin_dir: &Path, logs_dir: &Path, mode: &str, request: &Value) -> Result<Value> {
    let log = logs_dir.join(format!("provision-{mode}.err.log"));
    let mut child = Command::new(staged_exe(bin_dir, "baylee-dev-provision"))
        .arg(mode)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::from(File::create(&log)?))
        .spawn()
        .context("spawning baylee-dev-provision")?;
    // A request is a few hundred bytes: it fits the pipe before the helper
    // has to read it, so writing first cannot block against its reply.
    let written = child
        .stdin
        .take()
        .context("the helper's stdin")?
        .write_all(&serde_json::to_vec(request)?);
    let output = child.wait_with_output()?;
    written.context("handing the helper its request")?;
    ensure!(
        output.status.success(),
        "provisioning {mode} failed; {} says why",
        log.display()
    );
    serde_json::from_slice(&output.stdout).context("the helper's reply is not JSON")
}
