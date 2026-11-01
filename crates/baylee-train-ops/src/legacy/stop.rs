//! Stop one identity-bound native guard controller; never match process names.
use crate::{files, process};
use anyhow::{Context, Result, ensure};
use rustix::process::{Pid, PidfdFlags, Signal, pidfd_open, pidfd_send_signal};
use serde_json::{Value, json};
use std::{
    path::Path,
    thread,
    time::{Duration, Instant},
};
fn matches(expected: &process::Identity) -> bool {
    process::identity(expected.pid).as_ref() == Some(expected)
}
fn child_finished(child: Option<&process::Identity>) -> Result<bool> {
    if let Some(child) = child {
        if let Some(current) = process::identity(child.pid) {
            ensure!(
                current == *child,
                "owned child PID was reused; refusing to inspect another process group"
            );
            return Ok(false);
        }
        return Ok(process::group_members(child.pid)?.is_empty());
    }
    Ok(true)
}
/// Signal only a verified native supervisor through its pinned kernel process handle.
/// # Errors
/// Rejects stale/foreign/self identities and refuses escalation when cleanup exceeds the bound.
pub fn stop(job: &Path, timeout_seconds: f64) -> Result<Value> {
    ensure!(
        timeout_seconds.is_finite() && timeout_seconds > 0.,
        "finite positive stop deadline required"
    );
    let receipt = files::read_json(job)?;
    ensure!(
        receipt["guard"] == "rust-owned-group-v1",
        "stop requires a native owned-group job receipt"
    );
    if !matches!(receipt["status"].as_str(), Some("starting" | "running")) {
        return Ok(json!({"status":"not_active","job":job,"signal_sent":false}));
    }
    let controller: process::Identity = serde_json::from_value(receipt["controller"].clone())
        .context("job lacks supervising process identity")?;
    ensure!(
        controller.pid > 1 && controller.pid != std::process::id(),
        "refusing self/system controller stop"
    );
    ensure!(matches(&controller), "controller identity is stale");
    let pid = Pid::from_raw(i32::try_from(controller.pid)?).context("controller PID")?;
    let pinned = pidfd_open(pid, PidfdFlags::empty())?;
    ensure!(
        matches(&controller),
        "controller identity changed while pinning"
    );
    let child: Option<process::Identity> = serde_json::from_value(
        receipt
            .get("child_identity")
            .cloned()
            .unwrap_or(Value::Null),
    )?;
    if let Some(ref child) = child {
        ensure!(
            receipt["pid"] == child.pid && child.pid != controller.pid,
            "owned child identity differs from receipt"
        );
    }
    let current = files::read_json(job)?;
    ensure!(
        current["controller"] == receipt["controller"] && current["guard"] == receipt["guard"],
        "job ownership changed before stop"
    );
    if !matches!(current["status"].as_str(), Some("starting" | "running")) {
        return Ok(json!({"status":"not_active","job":job,"signal_sent":false}));
    }
    pidfd_send_signal(&pinned, Signal::TERM)?;
    let started = Instant::now();
    loop {
        let controller_gone = !matches(&controller);
        if controller_gone && child_finished(child.as_ref())? {
            return Ok(
                json!({"status":"stopped","job":job,"controller":controller,"child_identity":child,"signal_sent":true,"signal":"TERM","elapsed_seconds":started.elapsed().as_secs_f64()}),
            );
        }
        ensure!(
            started.elapsed().as_secs_f64() < timeout_seconds,
            "guard cleanup deadline expired; no escalation was sent"
        );
        thread::sleep(Duration::from_millis(25));
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn foreign_inactive_and_self_receipts_cannot_signal() -> Result<()> {
        let root = tempfile::tempdir()?;
        let path = root.path().join("job.json");
        files::atomic_json(&path, &json!({"guard":"old-shell","status":"running"}))?;
        assert!(stop(&path, 1.).is_err());
        files::atomic_json(
            &path,
            &json!({"guard":"rust-owned-group-v1","status":"complete"}),
        )?;
        assert_eq!(stop(&path, 1.)?["signal_sent"], false);
        let own = process::identity(std::process::id()).context("test identity")?;
        files::atomic_json(
            &path,
            &json!({"guard":"rust-owned-group-v1","status":"running","controller":own}),
        )?;
        assert!(stop(&path, 1.).is_err());
        Ok(())
    }
}
