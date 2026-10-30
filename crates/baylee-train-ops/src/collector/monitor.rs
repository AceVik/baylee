//! Quiet collector checks preserving existing event IDs and delivery ledgers.
use super::{attempts, json, pins, text};
use crate::{files, guard, process};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, path::Path, time::Duration};
fn receipts(root: &Path) -> Result<Vec<Value>> {
    let corrections_path = root.join("quota-corrections.json");
    let corrections = if corrections_path.exists() {
        let value = json(&corrections_path)?;
        let object = value.as_object().context("quota correction object")?;
        ensure!(
            object.len() == 2
                && value["schema_version"].as_u64() == Some(1)
                && value["corrections"].is_object(),
            "malformed quota corrections"
        );
        value["corrections"].clone()
    } else {
        json!({})
    };
    let mut applied = BTreeSet::new();
    for (name, correction) in corrections.as_object().context("corrections")? {
        let path = Path::new(name);
        ensure!(
            path.is_absolute()
                && path.file_name().is_some_and(|v| v == "receipt.json")
                && path
                    .canonicalize()?
                    .starts_with(root.join("attempts").canonicalize()?),
            "correction escapes collector receipts"
        );
        ensure!(
            correction.as_object().context("correction entry")?.len() == 3
                && correction["corrected_quota_limited"] == false
                && correction["receipt_sha256"].is_string(),
            "malformed correction entry"
        );
        ensure!(
            correction["evidence_sha256"]
                .as_object()
                .context("correction evidence")?
                .keys()
                .all(|name| Path::new(name).is_absolute()),
            "correction evidence must be absolute"
        );
        pins(&correction["evidence_sha256"])?;
        ensure!(
            files::sha256(path)? == text(correction, "receipt_sha256")?,
            "quota correction receipt changed"
        );
    }
    let mut rows = Vec::new();
    for path in attempts(root, "receipt.json")? {
        let mut row = json(&path)?;
        if let Some(correction) = corrections.get(path.to_str().context("receipt path UTF8")?) {
            ensure!(
                files::sha256(&path)? == text(correction, "receipt_sha256")?
                    && row["status"] == "admitted"
                    && row["technical"]["quota_limited"] == true,
                "correction must target admitted legacy true flag"
            );
            row["technical"]["quota_limited"] = json!(false);
            applied.insert(path.to_str().context("receipt path")?.to_owned());
        }
        rows.push(row);
    }
    ensure!(
        applied.len() == corrections.as_object().context("corrections")?.len(),
        "quota correction receipt missing"
    );
    Ok(rows)
}
type Event = (String, &'static str, Value);
fn findings(
    root: &Path,
    config: &Value,
    state: &mut Value,
    status: &Value,
    rows: &[Value],
    live: bool,
    now: f64,
) -> Result<Vec<Event>> {
    let mut events = Vec::new();
    if let Some(first) = rows.iter().find(|v| v["status"] == "admitted") {
        events.push((
            "first-admitted".into(),
            "First technically verified complete LLM collection game admitted",
            json!({"attempt_id":first["request"]["attempt_id"]}),
        ));
    }
    let mut pending = Vec::new();
    let blocked = root.join("transport-blocked.json");
    if blocked.exists() {
        pending.push(json!({"path":blocked,"status":"transport-blocked","detail":json(&blocked)?}));
    }
    for path in attempts(root, "room-state.json")? {
        let mut room = json(&path)?;
        if room["status"] != "closed" {
            room["path"] = json!(path);
            pending.push(room);
        }
    }
    if status["phase"] == "waiting-owned-room" && !pending.is_empty() {
        let mut paths: Vec<_> = pending
            .iter()
            .map(|v| v["path"].as_str().unwrap_or_default())
            .collect();
        paths.sort_unstable();
        events.push((
            format!("orphan:{}", paths.join(",")),
            "Collection blocked by unresolved owned room; no further table may start",
            json!({"rooms":pending}),
        ));
    }
    let floor = config
        .get("attempt_index_floor")
        .map(|v| v.as_u64().context("invalid attempt index floor"))
        .transpose()?
        .unwrap_or(0);
    if state["receipt_alert_floor"].as_u64().unwrap_or(0) != floor {
        state
            .as_object_mut()
            .context("monitor state")?
            .remove("quota_since");
    }
    state["receipt_alert_floor"] = json!(floor);
    let mut current = Vec::new();
    for row in rows {
        let index = if floor == 0 {
            0
        } else if let Some(value) = row["request"].get("index") {
            value.as_u64().context("invalid receipt attempt index")?
        } else {
            text(&row["request"], "attempt_id")?
                .strip_prefix("attempt-")
                .context("receipt lacks attempt index")?
                .parse::<u64>()?
        };
        if floor == 0 || index >= floor {
            current.push(row);
        }
    }
    let failures: Vec<_> = current
        .iter()
        .rev()
        .take_while(|v| v["status"] != "admitted")
        .collect();
    if failures.len() >= 2 {
        let first = &failures.last().context("failures")?["request"]["attempt_id"];
        events.push((format!("failures:{}",first.as_str().context("attempt id")?),"At least two consecutive collection attempts failed/incomplete",json!({"consecutive_failures":failures.len(),"first_attempt":first,"latest_attempt":failures[0]["request"]["attempt_id"]})));
    }
    let quota = current.last().is_some_and(|v| {
        v["technical"]["quota_limited"] == true
            || matches!(
                v["cause"].as_str(),
                Some("quota" | "limited" | "rate_limited")
            )
    });
    if quota {
        if state.get("quota_since").is_none() {
            state["quota_since"] = json!(now);
        }
        let since = state["quota_since"].as_f64().context("quota timestamp")?;
        if now - since >= 1800.0 {
            events.push((
                format!("quota:{}", state["quota_since"]),
                "Collector has remained quota-limited for at least30 minutes",
                json!({"since_unix":since}),
            ));
        }
    } else {
        state
            .as_object_mut()
            .context("state")?
            .remove("quota_since");
    }
    events.extend(controller_findings(root, config, state, status, live)?);
    Ok(events)
}
/// Perform one read-only collector check and queue only new actionable events.
/// # Errors
/// Refuses malformed config/ledger or changed pinned wake-helper bytes.
pub fn check(root: &Path) -> Result<String> {
    check_with(root, files::unix_time()?, |thread, message| {
        guard::capture(
            Path::new("/usr/local/bin/codex"),
            &["queue", "--thread", thread, "--message", message],
            Duration::from_secs(30),
        )
        .map(|_| ())
    })
}
fn check_with(
    root: &Path,
    now: f64,
    mut queue: impl FnMut(&str, &str) -> Result<()>,
) -> Result<String> {
    let root = root.canonicalize()?;
    let directory = root.join("monitor");
    if !directory.join("config.json").exists() {
        return Ok("unarmed".into());
    }
    let Some(_lock) = super::try_lock(&directory.join("lock"))? else {
        return Ok("busy".into());
    };
    let config = json(directory.join("config.json"))?;
    uuid::Uuid::parse_str(text(&config, "thread")?)?;
    ensure!(
        files::sha256(Path::new(text(&config, "watch_module")?))? == text(&config, "watch_sha256")?,
        "pinned wake helper changed"
    );
    let path = directory.join("state.json");
    let mut state = if path.exists() {
        json(&path)?
    } else {
        json!({})
    };
    ensure!(state.is_object(), "corrupt monitor ledger");
    for name in ["delivered", "retry_after"] {
        if state.get(name).is_none() {
            state[name] = json!({});
        }
        ensure!(state[name].is_object(), "corrupt delivery ledger");
    }
    if now < state["next_check"].as_f64().unwrap_or(0.0) {
        return Ok("waiting".into());
    }
    state["next_check"] = json!(now + 30.0);
    let result = (|| -> Result<Vec<Event>> {
        let status = json(root.join("status.json"))?;
        let rows = receipts(&root)?;
        let identity: process::Identity = serde_json::from_value(config["controller"].clone())?;
        findings(
            &root,
            &config,
            &mut state,
            &status,
            &rows,
            process::identity(identity.pid).as_ref() == Some(&identity),
            now,
        )
    })();
    let events = if let Ok(events) = result {
        state["read_errors"] = json!(0);
        events
    } else {
        let count = state["read_errors"].as_u64().unwrap_or(0) + 1;
        state["read_errors"] = json!(count);
        if count >= 3 {
            vec![(
                "persistent-read-error".into(),
                "Collector monitoring metadata remained unreadable",
                json!({"error":"ValueError"}),
            )]
        } else {
            Vec::new()
        }
    };
    files::atomic_json(&path, &state)?;
    let mut delivered = false;
    for (key, message, evidence) in events {
        let id = files::hex(&Sha256::digest(
            format!("{}\n{key}", root.display()).as_bytes(),
        ))[..24]
            .to_owned();
        if state["delivered"].get(&id).is_some()
            || now < state["retry_after"][&id].as_f64().unwrap_or(0.0)
        {
            continue;
        }
        let payload = json!({"event_id":id,"root":root,"finding":message,"evidence":evidence});
        let message = format!(
            "Collector actionable event {id}\n{}",
            crate::watch::state::canonical(&payload)?
        );
        if queue(text(&config, "thread")?, &message).is_err() {
            state["retry_after"][&id] = json!(now + 300.0);
        } else {
            let mut item = payload;
            item["time"] = json!(now);
            state["delivered"][&id] = item;
            state["retry_after"]
                .as_object_mut()
                .context("retry ledger")?
                .remove(&id);
            delivered = true;
        }
        files::atomic_json(&path, &state)?;
    }
    Ok(if delivered { "notified" } else { "quiet" }.into())
}

fn controller_findings(
    root: &Path,
    config: &Value,
    state: &mut Value,
    status: &Value,
    live: bool,
) -> Result<Vec<Event>> {
    let mut events = Vec::new();
    let dead = if live {
        0
    } else {
        state["dead_polls"].as_u64().unwrap_or(0) + 1
    };
    state["dead_polls"] = json!(dead);
    let intentional =
        (root.join("STOP").exists() || root.join("DRAIN").exists()) && status["phase"] == "stopped";
    let identity = crate::watch::state::canonical(&config["controller"])?;
    if dead >= 2 && !intentional && status["phase"] != "failed" {
        events.push((
            format!("dead:{identity}"),
            "Collection controller exited or PID identity changed",
            json!({"controller":config["controller"],"phase":status["phase"]}),
        ));
    }
    if status["phase"] == "failed" {
        events.push((
            format!("controller-failed:{identity}"),
            "Collection controller reports failure",
            json!({"phase":"failed"}),
        ));
    }
    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn historical_timestamp_decimals_survive_repeated_state_writes() -> Result<()> {
        let root = tempfile::tempdir()?;
        let path = root.path().join("state.json");
        let decimals = [
            "1793160306.1167815",
            "1793169226.6394777",
            "1793194503.6600087",
            "1793240833.6509361",
            "1793211106.6450865",
            "1793201663.6447005",
        ];
        for decimal in decimals {
            std::fs::write(
                &path,
                format!("{{\"delivered\":{{\"historical\":{{\"time\":{decimal}}}}}}}"),
            )?;
            for _ in 0..3 {
                files::atomic_json(&path, &files::read_json(&path)?)?;
                ensure!(
                    std::fs::read_to_string(&path)?.contains(decimal),
                    "historical timestamp changed: {decimal}"
                );
            }
        }
        Ok(())
    }
    #[test]
    fn receipt_floor_does_not_repeat_historical_quota() -> Result<()> {
        let root = tempfile::tempdir()?;
        let mut state = json!({"quota_since":1,"receipt_alert_floor":0});
        let events = findings(
            root.path(),
            &json!({"attempt_index_floor":409,"controller":{}}),
            &mut state,
            &json!({"phase":"stopped"}),
            &[json!({"cause":"quota","request":{"attempt_id":"attempt-0001"}})],
            true,
            4000.0,
        )?;
        ensure!(events.is_empty() && state.get("quota_since").is_none());
        Ok(())
    }
    #[test]
    fn failed_delivery_retries_but_success_is_never_repeated() -> Result<()> {
        let root = tempfile::tempdir()?;
        let helper = root.path().join("watch.py");
        std::fs::write(&helper, "immutable historical helper")?;
        files::atomic_json(
            &root.path().join("monitor/config.json"),
            &json!({"thread":uuid::Uuid::now_v7(),"watch_module":helper,"watch_sha256":files::sha256(&helper)?,"controller":{"pid":u32::MAX,"boot_id":"old","start_ticks":1}}),
        )?;
        files::atomic_json(&root.path().join("status.json"), &json!({"phase":"failed"}))?;
        ensure!(check_with(root.path(), 1000.0, |_, _| anyhow::bail!("offline"))? == "quiet");
        let mut calls = 0;
        ensure!(
            check_with(root.path(), 1031.0, |_, _| {
                calls += 1;
                Ok(())
            })? == "quiet"
        );
        ensure!(calls == 0);
        ensure!(
            check_with(root.path(), 1301.0, |_, _| {
                calls += 1;
                Ok(())
            })? == "notified"
        );
        ensure!(calls == 1);
        ensure!(
            check_with(root.path(), 1400.0, |_, _| {
                calls += 1;
                Ok(())
            })? == "quiet"
        );
        ensure!(calls == 1);
        Ok(())
    }
}
