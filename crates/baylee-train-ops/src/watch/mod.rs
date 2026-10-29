//! Read-only experiment observation with durable at-least-once wake delivery.
pub(crate) mod state;
#[cfg(test)]
mod tests;

use crate::{
    files,
    process::{self, Identity},
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    path::{Component, Path},
    time::Duration,
};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Config {
    root: String,
    thread: String,
    controller: Identity,
    success_phases: Vec<String>,
    artifacts: Vec<String>,
    poll_seconds: f64,
    error_polls: u64,
    death_polls: u64,
}

fn validate(config: &Config, root: &Path) -> Result<()> {
    uuid::Uuid::parse_str(&config.thread)?;
    ensure!(
        Path::new(&config.root).canonicalize()? == root.canonicalize()?,
        "watch root binding mismatch"
    );
    ensure!(
        !config.success_phases.is_empty()
            && config.poll_seconds.is_finite()
            && config.poll_seconds >= 1.0
            && config.error_polls >= 2
            && config.death_polls >= 2,
        "invalid watch phases or confirmation bounds"
    );
    for name in &config.artifacts {
        ensure!(
            !name.is_empty()
                && Path::new(name)
                    .components()
                    .all(|v| matches!(v, Component::Normal(_))),
            "invalid artifact-relative name"
        );
    }
    Ok(())
}

/// Arm one immutable watch from JSON configuration matching the legacy watcher.
/// # Errors
/// Rejects invalid configuration, dead/recycled owners or conflicting prior watches.
pub fn arm(root: &Path, config: &Value) -> Result<()> {
    let parsed: Config = serde_json::from_value(config.clone())?;
    validate(&parsed, root)?;
    ensure!(
        process::identity(parsed.controller.pid).as_ref() == Some(&parsed.controller),
        "controller is not live; refusing to arm"
    );
    let directory = root.join("watchdog");
    let _lock = files::lock(&directory.join("lock"))?;
    let path = directory.join("config.json");
    if path.exists() {
        ensure!(
            files::read_json(&path)? == *config,
            "watcher already armed differently"
        );
        return Ok(());
    }
    files::atomic_json(&directory.join("state.json"), &json!({}))?;
    files::atomic_json(&path, &config)
}

/// Inspect a run once and deliver a wake only after its terminal evidence is ready.
/// # Errors
/// Rejects corrupt configuration/state; queue failures persist for a later retry.
pub fn check(root: &Path) -> Result<String> {
    check_with(
        root,
        files::unix_time()?,
        process::identity,
        |thread, message| {
            crate::guard::capture(
                Path::new("/usr/local/bin/codex"),
                &["queue", "--thread", thread, "--message", message],
                Duration::from_secs(30),
            )
        },
    )
}

fn check_with(
    root: &Path,
    now: f64,
    identity: impl Fn(u32) -> Option<Identity>,
    mut queue: impl FnMut(&str, &str) -> Result<String>,
) -> Result<String> {
    let directory = root.join("watchdog");
    let lock = files::lock(&directory.join("lock"));
    let _lock = match lock {
        Ok(lock) => lock,
        Err(error)
            if error.chain().any(|e| {
                e.downcast_ref::<std::fs::TryLockError>()
                    .is_some_and(|v| matches!(v, std::fs::TryLockError::WouldBlock))
            }) =>
        {
            return Ok("busy".into());
        }
        Err(error) => return Err(error),
    };
    let raw = files::read_json(&directory.join("config.json"))?;
    let config: Config = serde_json::from_value(raw.clone())?;
    validate(&config, root)?;
    let state_path = directory.join("state.json");
    let mut state = files::read_json(&state_path)?;
    ensure!(
        state.is_object(),
        "watcher state corrupted; preserve delivery history"
    );
    if !state["delivered"].is_null() {
        files::atomic_json(&directory.join("delivered.json"), &state["delivered"])?;
        return Ok("delivered".into());
    }
    if state["pending"].is_null() {
        if now < state["next_check"].as_f64().unwrap_or(0.0) {
            return Ok("waiting".into());
        }
        state["next_check"] = json!(now + config.poll_seconds);
        let result = files::read_json(&root.join("status.json")).and_then(|v| {
            ensure!(
                v.is_object() && v["phase"].is_string(),
                "status must have a string phase"
            );
            Ok(v)
        });
        let (status, error) = match result {
            Ok(value) => {
                state["status_errors"] = json!(0);
                (value, None)
            }
            Err(error) => {
                state["status_errors"] = json!(state["status_errors"].as_u64().unwrap_or(0) + 1);
                (Value::Null, Some(error.to_string()))
            }
        };
        let live = identity(config.controller.pid).as_ref() == Some(&config.controller);
        state["dead_polls"] = json!(if live {
            0
        } else {
            state["dead_polls"].as_u64().unwrap_or(0) + 1
        });
        let finding = state::finding(&config, &state, &status, error.as_deref());
        if let Some(finding) = finding {
            state["pending"] = json!({"event_id":state::event_id(&raw)?,"finding":finding,"status":status,"status_error":error});
        }
        files::atomic_json(&state_path, &state)?;
        if state["pending"].is_null() {
            return Ok(if live { "running" } else { "confirming-exit" }.into());
        }
    } else if now < state["retry_at"].as_f64().unwrap_or(0.0) {
        return Ok("backoff".into());
    }
    let pending = &state["pending"];
    let event = pending["event_id"]
        .as_str()
        .context("missing pending event identity")?
        .to_owned();
    let message = format!(
        "Experiment wake event {event} (at-least-once delivery; deduplicate this event ID). Run root: {}. Finding: {}. Exact observed status.json: {}. Status read error: {}. Review the actual results or failure and continue the authorized work. Recheck current status before acting, respect existing user authorization, and avoid duplicate starts if another controller is still running.",
        config.root,
        pending["finding"].as_str().context("missing finding")?,
        pending["status"],
        pending["status_error"]
    );
    state["retry_at"] = json!(now + 60.0);
    files::atomic_json(&state_path, &state)?;
    match queue(&config.thread, &message) {
        Ok(receipt) => {
            state["delivered"] = json!({"event_id":event,"at":now,"queue_receipt":receipt});
            state
                .as_object_mut()
                .context("state object")?
                .remove("queue_error");
            files::atomic_json(&state_path, &state)?;
            files::atomic_json(&directory.join("delivered.json"), &state["delivered"])?;
            Ok("delivered".into())
        }
        Err(error) => {
            state["queue_error"] = json!(error.to_string());
            files::atomic_json(&state_path, &state)?;
            Ok("queue-retry".into())
        }
    }
}
