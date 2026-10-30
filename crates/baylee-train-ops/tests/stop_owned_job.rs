//! Stop one real bounded native supervisor and verify its owned group is empty.
#![cfg(target_os = "linux")]
use anyhow::{Context, Result, ensure};
use baylee_train_ops::{files, process};
use std::{
    path::Path,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};
struct OwnedController(Child);
impl Drop for OwnedController {
    fn drop(&mut self) {
        // The native guard itself has an eight-second deadline and cleans its
        // owned child group, including if this assertion unwinds early.
        let _ = self.0.wait();
    }
}
fn running(path: &Path) -> Result<serde_json::Value> {
    let started = Instant::now();
    loop {
        if let Ok(receipt) = files::read_json(path)
            && receipt["status"] == "running"
        {
            return Ok(receipt);
        }
        ensure!(
            started.elapsed() < Duration::from_secs(5),
            "native test guard did not publish running state"
        );
        thread::sleep(Duration::from_millis(20));
    }
}
#[test]
fn pidfd_stop_targets_only_one_guard_and_verifies_owned_child_cleanup() -> Result<()> {
    let root = tempfile::tempdir()?;
    let job = root.path().join("job");
    let child = Command::new(env!("CARGO_BIN_EXE_train_ops"))
        .args(["run", "--job"])
        .arg(&job)
        .args([
            "--timeout",
            "8",
            "--memory-gib",
            "1",
            "--",
            "/usr/bin/sleep",
            "60",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let mut controller = OwnedController(child);
    let receipt = running(&job.join("job.json"))?;
    let identity: process::Identity = serde_json::from_value(receipt["controller"].clone())?;
    ensure!(
        identity.pid == controller.0.id(),
        "guard controller identity differs"
    );
    let owned: process::Identity = serde_json::from_value(receipt["child_identity"].clone())
        .context("guard child identity")?;
    let mut stale = receipt;
    stale["controller"]["start_ticks"] = serde_json::json!(identity.start_ticks + 1);
    let stale_path = root.path().join("stale.json");
    files::atomic_json(&stale_path, &stale)?;
    let refused = Command::new(env!("CARGO_BIN_EXE_stop_job4"))
        .arg("--job")
        .arg(stale_path)
        .output()?;
    ensure!(
        !refused.status.success(),
        "stale controller identity was accepted"
    );
    ensure!(
        process::identity(identity.pid).as_ref() == Some(&identity),
        "stale stop touched the live controller"
    );
    let stopped = Command::new(env!("CARGO_BIN_EXE_stop_job4"))
        .args(["--job"])
        .arg(job.join("job.json"))
        .args(["--timeout", "5"])
        .output()?;
    ensure!(
        stopped.status.success(),
        "stop_job4 failed: {}",
        String::from_utf8_lossy(&stopped.stderr)
    );
    let result: serde_json::Value = serde_json::from_slice(&stopped.stdout)?;
    assert_eq!(result["status"], "stopped");
    assert!(!controller.0.wait()?.success());
    assert!(process::identity(owned.pid).is_none());
    assert!(process::group_members(owned.pid)?.is_empty());
    assert_eq!(
        files::read_json(&job.join("job.json"))?["reason"],
        "interrupted"
    );
    Ok(())
}
