use super::{Identity, Result, Value, event, identity, json, object, token};
use anyhow::{Context, ensure};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
    time::Duration,
};

pub(super) fn observe(
    config: &Value,
    state: &mut Value,
    current: &impl Fn(u32) -> Option<Identity>,
) -> Result<()> {
    if config.is_null() {
        return Ok(());
    }
    // Failed/incomplete process accounting means not ready, never permission to clean.
    if !ready(config, current).unwrap_or(false) {
        return Ok(());
    }
    let key = format!("maintenance-ready:{}", token(config)?);
    let controllers = config["controllers"].as_array().context("controllers")?;
    event(
        state,
        &key,
        "dedicated collection gateway 28768",
        "All collection controllers and seat clients stopped; review owned orphan cleanup and dedicated gateway maintenance; preserve the user gateway on port 28766 and GPU training; recheck process identities",
        json!({"controller_pids":controllers.iter().map(|r|r["controller"]["pid"].clone()).collect::<Vec<_>>(),"gateway_pid":config["gateway"]["pid"]}),
    );
    Ok(())
}

fn ready(config: &Value, current: &impl Fn(u32) -> Option<Identity>) -> Result<bool> {
    let controllers = config["controllers"].as_array().context("controllers")?;
    let units = controllers
        .iter()
        .filter_map(|r| r["unit"].as_str())
        .collect::<BTreeSet<_>>();
    ensure!(
        controllers.len() == 3 && units.len() == 3,
        "maintenance requires three distinct collector units"
    );
    let gateway = identity(&config["gateway"]).context("gateway identity")?;
    if current(gateway.pid).as_ref() != Some(&gateway) {
        return Ok(false);
    }
    for run in controllers {
        let owner = identity(&run["controller"]).context("collector identity")?;
        if current(owner.pid).is_some() {
            return Ok(false);
        }
        if !closed_unit(run["unit"].as_str().context("unit")?)? {
            return Ok(false);
        }
        let root = Path::new(run["root"].as_str().context("collector root")?);
        let status = object(&root.join("status.json")).context("collector status")?;
        let receipt = object(&root.join("controller.json")).context("collector receipt")?;
        if !matches!(status["phase"].as_str(), Some("failed" | "stopped"))
            || receipt["pid"].as_u64() != Some(u64::from(owner.pid))
            || !matches!(
                receipt["status"].as_str(),
                Some("exited" | "failed" | "stopped")
            )
        {
            return Ok(false);
        }
    }
    Ok(true)
}

fn closed_unit(unit: &str) -> Result<bool> {
    ensure!(
        unit.starts_with("baylee-")
            && unit.ends_with(".service")
            && unit
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.@".contains(&b)),
        "invalid maintenance unit"
    );
    let result = crate::guard::capture(
        Path::new("/usr/bin/systemctl"),
        &[
            "--user",
            "show",
            unit,
            "--property=ActiveState,MainPID,ControlGroup",
        ],
        Duration::from_secs(5),
    )?;
    let props = result
        .lines()
        .filter_map(|s| s.split_once('='))
        .collect::<BTreeMap<_, _>>();
    if !matches!(props.get("ActiveState"), Some(&"inactive" | &"failed"))
        || props.get("MainPID") != Some(&"0")
    {
        return Ok(false);
    }
    let group = props.get("ControlGroup").context("cgroup property")?;
    if group.is_empty() {
        return Ok(true);
    }
    let directory = Path::new("/sys/fs/cgroup")
        .join(group.trim_start_matches('/'))
        .canonicalize()?;
    ensure!(
        directory.starts_with("/sys/fs/cgroup"),
        "cgroup escapes root"
    );
    empty_group(&directory)
}

fn empty_group(path: &Path) -> Result<bool> {
    if !fs::read_to_string(path.join("cgroup.procs"))?
        .trim()
        .is_empty()
    {
        return Ok(false);
    }
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() && !empty_group(&entry.path())? {
            return Ok(false);
        }
    }
    Ok(true)
}
