//! Explicit monitor arming; never starts or modifies a collection controller.
use super::json;
use crate::{files, process};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::path::Path;
/// Bind an otherwise identical monitor config to one live collector identity.
/// # Errors
/// Refuses unrelated existing configuration or a non-running collector.
pub fn arm(root: &Path, pid: u32, thread: &str, watch: &Path) -> Result<Value> {
    uuid::Uuid::parse_str(thread)?;
    let root = root.canonicalize()?;
    let identity = process::identity(pid).context("cannot arm a non-live controller")?;
    let controller = json(root.join("controller.json"))?;
    ensure!(
        controller["pid"].as_u64() == Some(u64::from(pid))
            && matches!(controller["status"].as_str(), Some("running" | "setup")),
        "PID is not running collector"
    );
    let config = json!({"root":root,"thread":thread,"controller":identity,"watch_module":watch,"watch_sha256":files::sha256(watch)?});
    let directory = root.join("monitor");
    let _lock = files::lock(&directory.join("lock"))?;
    let path = directory.join("config.json");
    ensure!(
        !path.exists() || json(&path)? == config,
        "existing monitor identity differs; explicit reviewed rearm required"
    );
    files::atomic_json(&path, &config)?;
    Ok(config)
}
