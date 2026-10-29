//! Private credential bridge. Secrets remain in process memory/environment only.
use super::{json, pins, text};
use crate::{guard, process};
use anyhow::{Context, Result, ensure};
use std::{
    collections::BTreeMap,
    fs,
    os::unix::{fs::MetadataExt, process::CommandExt},
    path::Path,
    process::Command,
    time::{Duration, Instant},
};
fn argument<'a>(argv: &'a [String], flag: &str) -> Result<&'a str> {
    let indices: Vec<_> = argv
        .iter()
        .enumerate()
        .filter(|(_, v)| *v == flag)
        .collect();
    ensure!(
        indices.len() == 1,
        "collector argument missing or duplicated"
    );
    argv.get(indices[0].0 + 1)
        .map(String::as_str)
        .context("collector argument value")
}
fn prepare(plan: &serde_json::Value) -> Result<Command> {
    pins(&plan["sha256"])?;
    let argv: Vec<String> = serde_json::from_value(plan["argv"].clone())?;
    ensure!(
        !argv.is_empty() && Path::new(&argv[0]).is_absolute(),
        "collector executable must be absolute"
    );
    ensure!(
        text(plan, "gateway_service")? == "baylee-llm-teacher-gateway.service",
        "unexpected private gateway unit"
    );
    let output = guard::capture(
        Path::new("/usr/bin/systemctl"),
        &[
            "--user",
            "show",
            "baylee-llm-teacher-gateway.service",
            "-p",
            "MainPID",
            "-p",
            "ActiveState",
        ],
        Duration::from_secs(5),
    )?;
    let properties: BTreeMap<_, _> = output.lines().filter_map(|s| s.split_once('=')).collect();
    ensure!(
        properties.get("ActiveState") == Some(&"active"),
        "private gateway not active"
    );
    let pid: u32 = properties
        .get("MainPID")
        .context("gateway supervisor PID")?
        .parse()?;
    let supervisor = process::identity(pid).context("gateway supervisor not live")?;
    let state = json(text(plan, "gateway_state")?)?;
    ensure!(
        state["status"] == "ready"
            && state["gateway"] == argument(&argv, "--gateway")?
            && state["admin_url"] == argument(&argv, "--admin-url")?,
        "gateway endpoint or state differs"
    );
    let pid = u32::try_from(state["gateway_pid"].as_u64().context("gateway child PID")?)?;
    let before = process::identity(pid).context("gateway child not live")?;
    let directory = Path::new("/proc").join(pid.to_string());
    let stat = fs::read_to_string(directory.join("stat"))?;
    let fields = process::stat_fields(&stat)?;
    ensure!(
        fields
            .get(1)
            .context("gateway parent PID")?
            .parse::<u32>()?
            == supervisor.pid
            && fs::metadata(&directory)?.uid() == rustix::process::getuid().as_raw(),
        "gateway child ownership differs"
    );
    ensure!(
        directory.join("exe").canonicalize()?
            == Path::new(argument(&argv, "--gateway-bin")?).canonicalize()?,
        "gateway executable differs"
    );
    // Never format or propagate environment values into errors.
    let environment = fs::read(directory.join("environ"))?;
    let entries: BTreeMap<&[u8], &[u8]> = environment
        .split(|b| *b == 0)
        .filter_map(|e| {
            e.iter()
                .position(|b| *b == b'=')
                .map(|i| (&e[..i], &e[i + 1..]))
        })
        .collect();
    let token = entries
        .get(b"BAYLEE_ADMIN_TOKEN".as_slice())
        .context("private credential unavailable")?;
    let bind = entries
        .get(b"BAYLEE_ADMIN_BIND".as_slice())
        .context("private bind unavailable")?;
    ensure!(
        token.len() >= 32
            && token.is_ascii()
            && state["admin_url"]
                == format!(
                    "http://{}",
                    std::str::from_utf8(bind)
                        .map_err(|_| anyhow::anyhow!("private bind invalid"))?
                ),
        "private admin configuration invalid"
    );
    ensure!(
        process::identity(pid).as_ref() == Some(&before)
            && process::identity(supervisor.pid).as_ref() == Some(&supervisor),
        "gateway identity changed"
    );
    let mut command = Command::new(&argv[0]);
    command.args(&argv[1..]).env(
        "BAYLEE_COLLECTOR_ADMIN_TOKEN",
        std::str::from_utf8(token).map_err(|_| anyhow::anyhow!("private credential invalid"))?,
    );
    Ok(command)
}
/// Validate pinned launch inputs and replace this process with the collector.
/// # Errors
/// Emits one opaque error after a bounded wait; never reveals credential bytes.
pub fn run(plan: &Path) -> Result<()> {
    let result = (|| -> Result<()> {
        let path = plan;
        let plan = json(path)?;
        if plan.get("not_before_unix").is_some() {
            probe_preflight(path)?;
        }
        let deadline = Instant::now() + Duration::from_secs(45);
        let mut command = loop {
            match prepare(&plan) {
                Ok(command) => break command,
                Err(_) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(250));
                }
                Err(_) => anyhow::bail!("collector preflight refused"),
            }
        };
        Err(command.exec().into())
    })();
    result.map_err(|_| {
        anyhow::anyhow!("collector preflight failed; inspect non-secret identities and pinned plan")
    })
}

/// Validate the reset probe's quota window, fresh namespace and pinned inputs.
/// This command does not launch anything or reserve a room.
/// # Errors
/// Refuses a preexisting namespace, pending quota window or changed input.
pub fn probe_preflight(plan_path: &Path) -> Result<()> {
    let plan = json(plan_path)?;
    let hold = plan["not_before_unix"]
        .as_f64()
        .context("probe quota hold required")?;
    ensure!(
        hold.is_finite() && hold >= 0.0 && crate::files::unix_time()? >= hold,
        "quota hold has not elapsed"
    );
    let root = plan_path.parent().context("probe plan directory")?;
    ensure!(
        ["controller.json", "status.json", "attempts"]
            .iter()
            .all(|name| !root.join(name).exists()),
        "fresh namespace only; no duplicate attempts"
    );
    pins(&plan["sha256"])
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn reset_probe_hold_and_existing_namespace_fail_closed() -> Result<()> {
        let root = tempfile::tempdir()?;
        let pin = root.path().join("input");
        fs::write(&pin, "frozen input")?;
        let path = root.path().join("plan.json");
        let mut plan = json!({"not_before_unix":crate::files::unix_time()?+3600.0,"sha256":{pin.to_str().context("path")?:crate::files::sha256(&pin)?}});
        crate::files::atomic_json(&path, &plan)?;
        ensure!(probe_preflight(&path).is_err());
        plan["not_before_unix"] = json!(0);
        crate::files::atomic_json(&path, &plan)?;
        probe_preflight(&path)?;
        fs::create_dir(root.path().join("attempts"))?;
        ensure!(probe_preflight(&path).is_err());
        Ok(())
    }
}
