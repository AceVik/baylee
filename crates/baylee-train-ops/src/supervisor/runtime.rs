use super::{Manifest, Stage, check, owned::Owned};
use crate::{files, process};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::Read,
    net::TcpListener,
    os::unix::process::CommandExt,
    path::Path,
    process::{Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
fn health(manifest: &Manifest) -> Result<Value> {
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(2)))
        .build()
        .new_agent();
    let mut response = agent
        .get(format!("http://{}/health", manifest.bind))
        .call()?;
    Ok(response.body_mut().read_json()?)
}
fn ready(manifest: &Manifest, kind: &str) -> Result<bool> {
    if kind == "none" {
        return Ok(true);
    }
    let value = health(manifest)?;
    Ok(if kind == "gateway" {
        value["ok"] == true
    } else {
        value["agents"]["connected"].as_u64().is_some_and(|n| n > 0)
    })
}
fn spawn(manifest: &Manifest, stage: &Stage, token: &str) -> Result<Owned> {
    let log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(manifest.root.join(format!("{}.log", stage.name)))?;
    let mut command = Command::new(&stage.command[0]);
    command
        .args(&stage.command[1..])
        .current_dir(&manifest.root)
        .env_clear()
        .envs(&manifest.environment)
        .stdin(Stdio::null())
        .stdout(log.try_clone()?)
        .stderr(log)
        .process_group(0);
    for key in ["PATH", "HOME", "USER", "LANG", "LC_ALL", "LD_LIBRARY_PATH"] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    if stage.agent_token {
        command.env("BAYLEE_AGENT_TOKEN", token);
    }
    Owned::new(command.spawn()?)
}
fn publish(manifest: &Manifest, children: &[(String, Owned)], phase: &str) -> Result<()> {
    let identities: BTreeMap<_, _> = children
        .iter()
        .map(|(name, child)| (name, &child.identity))
        .collect();
    files::atomic_json(
        &manifest.root.join("status.json"),
        &json!({"phase":phase,"identity":process::identity(std::process::id()),"children":identities,"updated_unix":files::unix_time()?,"port":manifest.bind.port(),"guard":"rust-owned-supervisor-v1","automatic_restart":false}),
    )
}
fn wait_ready(
    manifest: &Manifest,
    stage: &Stage,
    children: &[(String, Owned)],
    stop: &AtomicBool,
) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs_f64(stage.readiness_seconds);
    loop {
        ensure!(!stop.load(Ordering::Relaxed), "supervisor interrupted");
        for (_, child) in children {
            child.alive()?;
        }
        if ready(manifest, &stage.readiness).unwrap_or(false) {
            return Ok(());
        }
        ensure!(
            Instant::now() < deadline,
            "service readiness deadline exceeded"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}
/// Run one pinned local host until interrupted; never restarts a child.
/// # Errors
/// Reports opaque failure after owned child cleanup. Does not alter other hosts.
pub fn run(path: &Path, stop: &AtomicBool) -> Result<()> {
    let manifest = check(path)?;
    let _lock = files::lock(&manifest.root.join("controller.lock"))?;
    let listener = TcpListener::bind(manifest.bind).context("local listener already owned")?;
    let mut secret = [0u8; 32];
    File::open("/dev/urandom")?.read_exact(&mut secret)?;
    let token = files::hex(&secret);
    drop(listener);
    let receipt = json!({"identity":process::identity(std::process::id()),"manifest_sha256":files::sha256(path)?,"gateway":format!("http://{}",manifest.bind),"automatic_restart":false,"guard":"rust-owned-supervisor-v1"});
    files::atomic_json(
        &manifest
            .root
            .join("native-supervisor-starts")
            .join(format!("{}.json", uuid::Uuid::now_v7())),
        &receipt,
    )?;
    fs::write(
        manifest.root.join("controller.pid"),
        format!("{}\n", std::process::id()),
    )?;
    let mut children = Vec::new();
    let result = serve(&manifest, &mut children, &token, stop);
    if result.is_err() && !stop.load(Ordering::Relaxed) {
        publish(&manifest, &children, "failed")?;
    }
    let mut cleanup_error = None;
    for (_, child) in children.iter_mut().rev() {
        if let Err(error) = child.cleanup() {
            cleanup_error = Some(error);
        }
    }
    if let Some(error) = cleanup_error {
        return Err(error);
    }
    if stop.load(Ordering::Relaxed) {
        publish(&manifest, &children, "stopped")?;
        return Ok(());
    }
    result
        .map_err(|_| anyhow::anyhow!("local supervisor failed; inspect non-secret component logs"))
}
fn serve(
    manifest: &Manifest,
    children: &mut Vec<(String, Owned)>,
    token: &str,
    stop: &AtomicBool,
) -> Result<()> {
    publish(manifest, children, "starting")?;
    for stage in &manifest.stages {
        ensure!(!stop.load(Ordering::Relaxed), "supervisor interrupted");
        children.push((stage.name.clone(), spawn(manifest, stage, token)?));
        publish(manifest, children, "starting")?;
        wait_ready(manifest, stage, children, stop)?;
    }
    publish(manifest, children, "serving")?;
    let mut failures = 0;
    while !stop.load(Ordering::Relaxed) {
        for (_, child) in children.iter() {
            child.alive()?;
        }
        if manifest.health_failure_limit > 0 {
            if ready(manifest, "agent").unwrap_or(false) {
                failures = 0;
            } else {
                failures += 1;
            }
            ensure!(
                failures < manifest.health_failure_limit,
                "gateway health lost"
            );
        }
        let deadline = Instant::now() + Duration::from_secs_f64(manifest.poll_seconds);
        while !stop.load(Ordering::Relaxed) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(50));
        }
    }
    Ok(())
}
