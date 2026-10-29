use super::*;
use std::os::unix::process::CommandExt;
use std::{
    fs,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
fn manifest(root: &Path, command: Vec<String>) -> Result<PathBuf> {
    let socket = std::net::TcpListener::bind("127.0.0.1:0")?;
    let bind = socket.local_addr()?;
    drop(socket);
    let executable = PathBuf::from(&command[0]);
    let value = Manifest {
        version: 1,
        root: root.to_owned(),
        bind,
        environment: BTreeMap::from([
            ("PORT".into(), bind.port().to_string()),
            ("BAYLEE_BIND".into(), bind.ip().to_string()),
            ("BAYLEE_GATEWAY".into(), format!("http://{bind}")),
            ("BAYLEE_EP".into(), "cpu".into()),
            ("CUDA_VISIBLE_DEVICES".into(), String::new()),
            ("BAYLEE_ENGINE_BIN".into(), executable.display().to_string()),
        ]),
        sha256: BTreeMap::from([(executable.clone(), files::sha256(&executable)?)]),
        stages: vec![Stage {
            name: "fixture".into(),
            command,
            agent_token: false,
            readiness: "none".into(),
            readiness_seconds: 2.0,
        }],
        health_failure_limit: 0,
        poll_seconds: 0.05,
    };
    let path = root.join("manifest.json");
    files::atomic_json(&path, &value)?;
    Ok(path)
}
#[test]
fn check_refuses_literal_secrets_and_changed_pins() -> Result<()> {
    let root = tempfile::tempdir()?;
    let path = manifest(root.path(), vec!["/usr/bin/sleep".into(), "60".into()])?;
    check(&path)?;
    let mut value = files::read_json(&path)?;
    value["environment"]["BAYLEE_AGENT_TOKEN"] = serde_json::json!("must never be persisted");
    files::atomic_json(&path, &value)?;
    ensure!(check(&path).is_err());
    value["environment"]
        .as_object_mut()
        .context("environment")?
        .remove("BAYLEE_AGENT_TOKEN");
    value["sha256"]["/usr/bin/sleep"] = serde_json::json!("0".repeat(64));
    files::atomic_json(&path, &value)?;
    ensure!(check(&path).is_err());
    Ok(())
}
#[test]
fn real_owned_child_stops_and_preserves_other_processes() -> Result<()> {
    let root = tempfile::tempdir()?;
    let path = manifest(root.path(), vec!["/usr/bin/sleep".into(), "60".into()])?;
    let mut unrelated = owned::Owned::new(
        std::process::Command::new("/usr/bin/sleep")
            .arg("60")
            .process_group(0)
            .spawn()?,
    )?;
    let stop = AtomicBool::new(false);
    std::thread::scope(|scope| -> Result<()> {
        let handle = scope.spawn(|| run(&path, &stop));
        let deadline = Instant::now() + Duration::from_secs(5);
        let result = (|| -> Result<()> {
            loop {
                if let Ok(value) = files::read_json(&root.path().join("status.json"))
                    && value["phase"] == "serving"
                {
                    let pid = u32::try_from(
                        value["children"]["fixture"]["pid"]
                            .as_u64()
                            .context("child PID")?,
                    )?;
                    ensure!(process_alive(pid));
                    return Ok(());
                }
                ensure!(
                    Instant::now() < deadline,
                    "supervisor did not reach serving"
                );
                std::thread::sleep(Duration::from_millis(20));
            }
        })();
        stop.store(true, Ordering::Relaxed);
        handle
            .join()
            .map_err(|_| anyhow::anyhow!("supervisor panicked"))??;
        result
    })?;
    let status = files::read_json(&root.path().join("status.json"))?;
    ensure!(status["phase"] == "stopped");
    let pid = u32::try_from(
        status["children"]["fixture"]["pid"]
            .as_u64()
            .context("child PID")?,
    )?;
    ensure!(!process_alive(pid));
    unrelated.alive()?;
    unrelated.cleanup()?;
    Ok(())
}
fn process_alive(pid: u32) -> bool {
    crate::process::identity(pid).is_some()
}
#[test]
fn occupied_listener_and_owned_lock_prevent_duplicate_start() -> Result<()> {
    let root = tempfile::tempdir()?;
    let path = manifest(root.path(), vec!["/usr/bin/sleep".into(), "60".into()])?;
    let value = check(&path)?;
    let listener = std::net::TcpListener::bind(value.bind)?;
    ensure!(run(&path, &AtomicBool::new(false)).is_err());
    ensure!(!root.path().join("status.json").exists());
    drop(listener);
    let _lock = files::lock(&root.path().join("controller.lock"))?;
    ensure!(run(&path, &AtomicBool::new(false)).is_err());
    Ok(())
}
#[test]
fn child_failure_cleans_group_and_never_restarts() -> Result<()> {
    let root = tempfile::tempdir()?;
    let path = manifest(root.path(), vec!["/usr/bin/false".into()])?;
    ensure!(run(&path, &AtomicBool::new(false)).is_err());
    ensure!(files::read_json(&root.path().join("status.json"))?["phase"] == "failed");
    ensure!(fs::read_dir(root.path().join("native-supervisor-starts"))?.count() == 1);
    Ok(())
}
#[test]
fn exited_leader_remains_pinned_until_forked_child_group_is_empty() -> Result<()> {
    let mut owned = owned::Owned::new(
        std::process::Command::new("/bin/sh")
            .args(["-c", "sleep 60 & exit 0"])
            .process_group(0)
            .spawn()?,
    )?;
    let pid = owned.child.id();
    let deadline = Instant::now() + Duration::from_secs(3);
    while !owned.exited()? {
        ensure!(Instant::now() < deadline, "fixture did not exit");
        std::thread::sleep(Duration::from_millis(10));
    }
    owned.cleanup()?;
    ensure!(crate::process::group_members(pid)?.is_empty());
    Ok(())
}
