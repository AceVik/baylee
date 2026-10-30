//! Read-only controller/round monitoring, preserving the existing delivery ledger.
mod maintenance;
mod runs;
#[cfg(test)]
mod tests;
mod transitions;

use crate::{
    files,
    process::{self, Identity},
};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{path::Path, time::Duration};

fn object(path: &Path) -> Option<Value> {
    files::read_json(path).ok().filter(Value::is_object)
}
fn token(value: &Value) -> Result<String> {
    Ok(files::hex(&Sha256::digest(
        crate::watch::state::canonical(value)?.as_bytes(),
    ))[..16]
        .to_owned())
}
fn identity(value: &Value) -> Option<Identity> {
    let parsed: Identity = serde_json::from_value(value.clone()).ok()?;
    (parsed.pid > 1 && !parsed.boot_id.is_empty()).then_some(parsed)
}
fn event(state: &mut Value, id: &str, root: &str, finding: &str, detail: Value) {
    if state["pending"].get(id).is_none() && state["delivered"].get(id).is_none() {
        let mut item = json!({"root":root,"finding":finding});
        item["detail"] = detail;
        state["pending"][id] = item;
    }
}

/// Check configured runs once. Never changes a training or collection controller.
/// # Errors
/// Refuses malformed state/configuration rather than losing delivery identities.
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
    current: impl Fn(u32) -> Option<Identity>,
    mut queue: impl FnMut(&str, &str) -> Result<String>,
) -> Result<String> {
    let _lock = match files::lock(&root.join("lock")) {
        Ok(lock) => lock,
        Err(error)
            if error.chain().any(|e| {
                e.downcast_ref::<std::fs::TryLockError>()
                    .is_some_and(|e| matches!(e, std::fs::TryLockError::WouldBlock))
            }) =>
        {
            return Ok("busy".into());
        }
        Err(error) => return Err(error),
    };
    let config = files::read_json(&root.join("config.json"))?;
    let thread = config["thread"]
        .as_str()
        .context("monitor thread required")?;
    uuid::Uuid::parse_str(thread)?;
    let state_path = root.join("state.json");
    let mut state = if state_path.exists() {
        files::read_json(&state_path)?
    } else {
        json!({})
    };
    ensure!(
        state.is_object(),
        "corrupt monitor state; preserve delivery ledger"
    );
    for key in ["pending", "delivered", "observed"] {
        if state.get(key).is_none() {
            state[key] = json!({});
        }
        ensure!(state[key].is_object(), "corrupt monitor {key} ledger");
    }
    if now < state["next_check"].as_f64().unwrap_or(0.0) {
        return Ok("waiting".into());
    }
    state["next_check"] = json!(now + config["poll_seconds"].as_f64().unwrap_or(30.0).max(1.0));
    for name in ["training", "terra", "cpu_review"] {
        runs::observe(&config[name], name, &mut state, now, &current)?;
    }
    for prefix in ["flash-transition", "lane9-resume"] {
        transitions::observe(root, prefix, &mut state, now, &current)?;
    }
    transitions::probe(&config["flash_probe"], &mut state, now, &current);
    maintenance::observe(&config["maintenance"], &mut state, &current)?;
    files::atomic_json(&state_path, &state)?;
    deliver(&state_path, &mut state, thread, now, &mut queue)?;
    Ok("checked".into())
}

fn deliver(
    path: &Path,
    state: &mut Value,
    thread: &str,
    now: f64,
    queue: &mut impl FnMut(&str, &str) -> Result<String>,
) -> Result<()> {
    let pending = state["pending"]
        .as_object()
        .context("pending object")?
        .clone();
    for (id, item) in pending {
        if now < item["retry_at"].as_f64().unwrap_or(0.0) {
            continue;
        }
        let root = item["root"].as_str().context("pending event root")?;
        let finding = item["finding"].as_str().context("pending event finding")?;
        let message = format!(
            "Codex monitor event {id} (at-least-once; deduplicate this ID). Run root: {root}. {finding}. Evidence: {}. Review results and continue authorized work; recheck current process identity and status before acting. Do not start duplicate controllers.",
            item["detail"]
        );
        state["pending"][&id]["retry_at"] = json!(now + 60.0);
        files::atomic_json(path, state)?;
        match queue(thread, &message) {
            Ok(receipt) => {
                state["delivered"][&id] = json!({"at":now,"receipt":receipt,"finding":finding});
                state["pending"]
                    .as_object_mut()
                    .context("pending object")?
                    .remove(&id);
            }
            Err(error) => state["pending"][&id]["queue_error"] = json!(error.to_string()),
        }
        files::atomic_json(path, state)?;
    }
    Ok(())
}
