//! Durable one-shot handovers. Existing terminal receipts are never rearmed.
use super::{json, pins, text, validate_clean_probe, verify_closed};
use crate::{files, guard, process};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::Path,
    time::{Duration, Instant},
};
const PRIMARY: &str = "baylee-llm-teacher.service";
const LANE9: &str = "baylee-llm-flash-teacher.service";
#[derive(Clone)]
struct Unit {
    active: String,
    pid: u32,
    argv: Vec<String>,
}
fn unit(name: &str) -> Result<Unit> {
    let output = guard::capture(
        Path::new("/usr/bin/systemctl"),
        &[
            "--user",
            "show",
            name,
            "--property=ActiveState,MainPID,ExecStart",
        ],
        Duration::from_secs(5),
    )?;
    let property = |key: &str| -> Result<&str> {
        output
            .lines()
            .find_map(|s| s.strip_prefix(key))
            .context("missing unit property")
    };
    let exec = property("ExecStart=")?;
    let argv = exec
        .split_once("argv[]=")
        .context("unverifiable ExecStart")?
        .1
        .split_once(" ;")
        .context("unverifiable ExecStart terminator")?
        .0;
    // These reviewed plans use plain absolute paths/URLs. Refuse quoting and escapes
    // rather than silently parsing a different systemd command.
    ensure!(
        !argv.chars().any(|c| ['\\', '\'', '"'].contains(&c)),
        "complex systemd argv requires explicit reviewed migration"
    );
    Ok(Unit {
        active: property("ActiveState=")?.to_owned(),
        pid: property("MainPID=")?.parse()?,
        argv: argv.split_whitespace().map(str::to_owned).collect(),
    })
}
fn start(name: &str) -> Result<()> {
    guard::capture(
        Path::new("/usr/bin/systemctl"),
        &["--user", "start", name],
        Duration::from_secs(8),
    )?;
    Ok(())
}
fn verify_primary(config: &Value, unit: &Unit) -> Result<()> {
    ensure!(
        text(config, "unit")? == PRIMARY
            && serde_json::to_value(&unit.argv)? == config["expected_argv"],
        "reviewed primary ExecStart differs"
    );
    let argv = &unit.argv;
    ensure!(
        argv.first().map(String::as_str) == Some(text(config, "binary")?)
            && argv.get(1).is_some_and(|v| v == "collect"),
        "unexpected collector command"
    );
    let root = text(config, "root")?;
    for (flag, value) in [
        ("--root", root.to_owned()),
        ("--profiles-config", format!("{root}/profiles-flash.json")),
        ("--pairings-file", format!("{root}/pairings-flash.json")),
        ("--concurrency", "34".into()),
    ] {
        let positions: Vec<_> = argv
            .iter()
            .enumerate()
            .filter(|(_, v)| *v == flag)
            .collect();
        ensure!(
            positions.len() == 1 && argv.get(positions[0].0 + 1) == Some(&value),
            "required collector argument differs"
        );
        if flag != "--root" && flag != "--concurrency" {
            ensure!(
                config["sha256"][&value].is_string(),
                "required input not pinned"
            );
        }
    }
    ensure!(
        config["sha256"][text(config, "binary")?].is_string(),
        "binary not pinned"
    );
    pins(&config["sha256"])
}
fn state(path: &Path) -> Result<Value> {
    if path.exists() {
        json(path)
    } else {
        Ok(json!({}))
    }
}
fn terminal(state: &Value, lane: bool) -> Option<String> {
    state["phase"]
        .as_str()
        .filter(|p| ["done", "failed"].contains(p) || lane && *p == "blocked")
        .map(str::to_owned)
}
fn bind_monitor(root: &Path, old: &Value, controller: &Value, floor: Option<u64>) -> Result<()> {
    let _lock = files::lock(&root.join("monitor/lock"))?;
    let path = root.join("monitor/config.json");
    let mut monitor = json(&path)?;
    ensure!(
        monitor["controller"] == *old || monitor["controller"] == *controller,
        "monitor controller ownership changed"
    );
    monitor["controller"] = controller.clone();
    if let Some(floor) = floor {
        monitor["attempt_index_floor"] = json!(floor);
    }
    files::atomic_json(&path, &monitor)
}
/// Finish the historical primary transition, once, preserving terminal receipts.
/// # Errors
/// Refuses changed pins/identity, unclosed attempts, rejected probe, or ambiguous recovery.
pub fn finish(ops: &Path) -> Result<String> {
    handover(ops, false)
}
/// Resume the reviewed historical lane9, once, preserving terminal receipts.
/// # Errors
/// Refuses stale intent, any old-PID reuse, changed pins, or failed admission.
pub fn resume(ops: &Path) -> Result<String> {
    handover(ops, true)
}
fn handover(ops: &Path, lane: bool) -> Result<String> {
    handover_with(ops, lane, &mut RealSystem)
}
fn handover_with(ops: &Path, lane: bool, system: &mut impl System) -> Result<String> {
    let stem = if lane {
        "lane9-resume"
    } else {
        "flash-transition"
    };
    let lock = ops.join(if lane {
        "lane9-resume.lock"
    } else {
        "transition.lock"
    });
    let Some(_lock) = super::try_lock(&lock)? else {
        return Ok("busy".into());
    };
    let path = ops.join(format!("{stem}-state.json"));
    let mut receipt = state(&path)?;
    if let Some(done) = terminal(&receipt, lane) {
        return Ok(done);
    }
    let config = json(ops.join(format!("{stem}.json")))?;
    let now = files::unix_time()?;
    let hold = config["not_before_unix"]
        .as_f64()
        .context("reviewed quota hold required")?;
    ensure!(hold.is_finite() && hold >= 0.0, "invalid quota hold");
    if now < hold {
        return Ok(if lane {
            "planned-quota-hold"
        } else {
            "waiting-quota-window"
        }
        .into());
    }
    verify_activation(ops, stem, &config, now, hold)?;
    ensure!(
        config["clean_probe_root"].is_string(),
        "native handover clean probe gate required"
    );
    if !validate_clean_probe(&config)? {
        return Ok("waiting-probe".into());
    }
    let name = text(&config, "unit")?;
    ensure!(
        name == if lane { LANE9 } else { PRIMARY },
        "unreviewed collector unit"
    );
    let root = Path::new(text(&config, "root")?);
    let old: process::Identity = serde_json::from_value(config["old_controller"].clone())?;
    if waiting_old(root, lane, &old, system)? {
        return Ok("waiting-old-controller".into());
    }
    let current = system.unit(name)?;
    if lane {
        ensure!(
            files::hex(&Sha256::digest(serde_json::to_vec(&current.argv)?))
                == text(&config, "argv_sha256")?,
            "reviewed lane9 argv changed"
        );
        pins(&config["sha256"])?;
    } else {
        verify_primary(&config, &current)?;
    }
    let fingerprint = files::sha256(&ops.join(format!("{stem}.json")))?;
    if receipt["phase"] == "starting" {
        ensure!(
            receipt["native_config_sha256"] == fingerprint,
            "handover intent belongs to another configuration"
        );
    } else {
        ensure!(
            current.pid == 0
                && current.active == "inactive"
                && json(root.join("status.json"))?["phase"] == "stopped",
            "collector is not cleanly inactive"
        );
        verify_closed(root)?;
        ensure!(root.join("DRAIN").is_file(), "DRAIN sentinel missing");
        bind_monitor(
            root,
            &config["old_controller"],
            &config["old_controller"],
            None,
        )?;
        receipt =
            json!({"phase":"starting","native_config_sha256":fingerprint,"old_controller":old});
        files::atomic_json(&path, &receipt)?;
    }
    let result = activate(ops, lane, &config, &path, &mut receipt, current, system);
    if result.is_err() {
        fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(root.join("DRAIN"))?;
        receipt["phase"] = json!("failed");
        files::atomic_json(&path, &receipt)?;
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn completed_handover_does_not_need_config_or_start() -> Result<()> {
        let root = tempfile::tempdir()?;
        for (name, lane) in [("flash-transition", false), ("lane9-resume", true)] {
            files::atomic_json(
                &root.path().join(format!("{name}-state.json")),
                &json!({"phase":"done","controller":{"pid":123}}),
            )?;
            ensure!(handover(root.path(), lane)? == "done");
        }
        Ok(())
    }
    #[test]
    fn quota_hold_precedes_probe_and_systemd_access() -> Result<()> {
        let root = tempfile::tempdir()?;
        files::atomic_json(
            &root.path().join("flash-transition.json"),
            &json!({"not_before_unix":files::unix_time()?+3600.0}),
        )?;
        ensure!(finish(root.path())? == "waiting-quota-window");
        Ok(())
    }
}

fn activate(
    ops: &Path,
    lane: bool,
    config: &Value,
    path: &Path,
    receipt: &mut Value,
    mut current: Unit,
    system: &mut impl System,
) -> Result<String> {
    let old: process::Identity = serde_json::from_value(config["old_controller"].clone())?;
    let root = Path::new(text(config, "root")?);
    let name = text(config, "unit")?;

    if current.pid == 0 {
        ensure!(
            current.active == "inactive" && json(root.join("status.json"))?["phase"] == "stopped",
            "ambiguous collector recovery"
        );
        verify_closed(root)?;
        let archive = ops.join(if lane {
            "lane9-drain-before-resume"
        } else {
            "DRAIN.before-flash"
        });
        if root.join("DRAIN").exists() {
            if archive.exists() {
                fs::remove_file(root.join("DRAIN"))?;
            } else {
                fs::rename(root.join("DRAIN"), &archive)?;
            }
        } else {
            ensure!(archive.is_file(), "drain archival evidence missing");
        }
        system.start(name)?;
    }
    let deadline = Instant::now() + Duration::from_secs(if lane { 30 } else { 10 });
    loop {
        current = system.unit(name)?;
        if !lane {
            verify_primary(config, &current)?;
        }
        let identity = system.identity(current.pid);
        if let Some(identity) = identity.filter(|v| v.pid != old.pid) {
            let value = serde_json::to_value(identity)?;
            if let Some(observed) = receipt.get("new_controller") {
                ensure!(*observed == value, "new controller identity changed");
            } else {
                receipt["new_controller"] = value.clone();
                files::atomic_json(path, &receipt)?;
            }
            if json(root.join("controller.json"))?.get("pid") == value.get("pid")
                && json(root.join("controller.json"))?["status"] == "running"
                && current.active == "active"
            {
                bind_monitor(
                    root,
                    &config["old_controller"],
                    &value,
                    if lane { None } else { Some(409) },
                )?;
                receipt["phase"] = json!("done");
                receipt["controller"] = value;
                files::atomic_json(path, &receipt)?;
                return Ok("done".into());
            }
        }
        ensure!(
            current.active != "failed" && Instant::now() < deadline,
            "collector did not become live"
        );
        std::thread::sleep(Duration::from_millis(200));
    }
}

fn verify_activation(ops: &Path, stem: &str, config: &Value, now: f64, hold: f64) -> Result<()> {
    // A new native start requires an explicit review receipt. Old scripts' earlier
    // quota windows are never sufficient authority to restart a collector.
    let activation = json(ops.join("collector-activation-plan.json"))?;
    ensure!(
        activation["schema_version"] == 1
            && activation["unit"] == config["unit"]
            && activation["not_before_unix"]
                .as_f64()
                .is_some_and(|v| v >= hold && now >= v)
            && activation["config_sha256"] == files::sha256(&ops.join(format!("{stem}.json")))?,
        "native collector activation plan differs or quota hold remains"
    );
    Ok(())
}

trait System {
    fn unit(&mut self, name: &str) -> Result<Unit>;
    fn start(&mut self, name: &str) -> Result<()>;
    fn identity(&self, pid: u32) -> Option<process::Identity>;
}
struct RealSystem;
impl System for RealSystem {
    fn unit(&mut self, name: &str) -> Result<Unit> {
        unit(name)
    }
    fn start(&mut self, name: &str) -> Result<()> {
        start(name)
    }
    fn identity(&self, pid: u32) -> Option<process::Identity> {
        process::identity(pid)
    }
}
#[cfg(test)]
#[path = "transition_tests.rs"]
mod lifecycle_tests;

fn waiting_old(
    root: &Path,
    lane: bool,
    old: &process::Identity,
    system: &impl System,
) -> Result<bool> {
    if lane {
        ensure!(
            old.pid == 373_342
                && root == Path::new("/home/ace/baylee-data/v5/llm-teacher-lane9-20261028"),
            "unexpected lane9 identity"
        );
        ensure!(system.identity(old.pid).is_none(), "old lane9 PID present");
    } else if system.identity(old.pid).as_ref() == Some(old) {
        return Ok(true);
    }
    Ok(false)
}
