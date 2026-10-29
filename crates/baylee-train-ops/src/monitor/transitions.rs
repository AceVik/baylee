use super::{Identity, Result, Value, event, identity, json, object, token};
use crate::files;
use sha2::{Digest, Sha256};
use std::path::Path;

pub(super) fn observe(
    root: &Path,
    prefix: &str,
    state: &mut Value,
    now: f64,
    current: &impl Fn(u32) -> Option<Identity>,
) -> Result<()> {
    let config_path = root.join(format!("{prefix}.json"));
    let result_path = root.join(format!("{prefix}-state.json"));
    if !config_path.exists() && !result_path.exists() {
        return Ok(());
    }
    let config = object(&config_path).unwrap_or(Value::Null);
    let result = object(&result_path).unwrap_or(Value::Null);
    let digest = if config.is_object() {
        token(&config)?
    } else {
        result["config_sha256"]
            .as_str()
            .unwrap_or("unreadable")
            .chars()
            .take(16)
            .collect()
    };
    let key = format!("{prefix}:{digest}");
    let prior = state["observed"][&key].clone();
    let mut observation =
        json!({"checked_at":now,"first_seen":prior["first_seen"].as_f64().unwrap_or(now)});
    let phase = result["phase"].as_str();
    let mut finding = None;
    if matches!(phase, Some("done" | "failed" | "blocked")) {
        finding = Some(if phase == Some("done") {
            "Flash transition completed"
        } else {
            "Flash transition failed"
        });
    } else if now < config["not_before_unix"].as_f64().unwrap_or(0.0) {
        observation["first_seen"] = config["not_before_unix"].clone();
        observation["planned_hold_until"] = config["not_before_unix"].clone();
    } else if waiting_probe(&config, phase) {
        observation["waiting_clean_probe"] = json!(true);
        observation["first_seen"] = json!(now);
    } else if !config.is_object() || (result_path.exists() && !result.is_object()) {
        let errors = prior["read_errors"].as_u64().unwrap_or(0) + 1;
        observation["read_errors"] = json!(errors);
        if errors >= 2 {
            finding = Some("Flash transition configuration or state persistently unreadable");
        }
    } else {
        if prefix == "lane9-resume"
            && now >= config["not_before_unix"].as_f64().unwrap_or(f64::INFINITY)
        {
            finding = Some("Flash lane9 quota hold ended; review safe resume readiness");
        }
        let live = identity(&config["old_controller"])
            .is_some_and(|p| current(p.pid).as_ref() == Some(&p));
        let dead = if live {
            0
        } else {
            prior["dead_polls"].as_u64().unwrap_or(0) + 1
        };
        observation["dead_polls"] = json!(dead);
        let collector = config["root"]
            .as_str()
            .and_then(|p| object(&Path::new(p).join("status.json")))
            .unwrap_or(Value::Null);
        observation["collector_phase"] = collector["phase"].clone();
        if dead >= 2 && phase != Some("starting") {
            finding = Some(
                "Flash transition stalled after old controller exited; inspect transition refusals",
            );
        } else if phase == Some("starting") {
            let since = prior["starting_since"].as_f64().unwrap_or(now);
            observation["starting_since"] = json!(since);
            if now - since >= 180.0 {
                finding = Some("Flash transition remains starting beyond its startup bound");
            }
        } else if now - observation["first_seen"].as_f64().unwrap_or(now)
            >= config["monitor_drain_timeout_seconds"]
                .as_f64()
                .unwrap_or(1800.0)
        {
            finding = Some("Flash transition drain has not completed within its monitoring bound");
        }
    }
    state["observed"][&key] = observation.clone();
    if let Some(finding) = finding {
        event(
            state,
            &format!("{key}:terminal"),
            &root.to_string_lossy(),
            finding,
            json!({"transition_state":result_path,"phase":result["phase"],"collector_phase":observation["collector_phase"]}),
        );
    }
    Ok(())
}

fn waiting_probe(config: &Value, phase: Option<&str>) -> bool {
    config["clean_probe_root"].as_str().is_some_and(|p| {
        phase != Some("starting")
            && !matches!(
                object(&Path::new(p).join("status.json"))
                    .as_ref()
                    .and_then(|v| v["phase"].as_str()),
                Some("stopped" | "failed")
            )
    })
}

pub(super) fn probe(
    config: &Value,
    state: &mut Value,
    now: f64,
    current: &impl Fn(u32) -> Option<Identity>,
) {
    let Some(root) = config["root"].as_str() else {
        return;
    };
    if now < config["not_before_unix"].as_f64().unwrap_or(0.0) {
        return;
    }
    let key = format!(
        "flash-probe:{}",
        &files::hex(&Sha256::digest(root.as_bytes()))[..16]
    );
    let prior = state["observed"][&key].clone();
    let controller = object(&Path::new(root).join("controller.json")).unwrap_or(Value::Null);
    let status = object(&Path::new(root).join("status.json")).unwrap_or(Value::Null);
    let actual = controller["pid"]
        .as_u64()
        .and_then(|p| u32::try_from(p).ok())
        .and_then(current);
    let bound = identity(&prior["controller"]).or_else(|| actual.clone());
    let live = actual.is_some() && actual == bound;
    let dead = if live {
        0
    } else {
        prior["dead_polls"].as_u64().unwrap_or(0) + 1
    };
    state["observed"][&key] =
        json!({"controller":bound,"dead_polls":dead,"phase":status["phase"],"checked_at":now});
    // Completion is a request for native technical review, never an automatic
    // admission. The observer cannot relax the collector's quota/replay gates.
    let finding = match status["phase"].as_str() {
        Some("failed") => Some("Flash quota probe failed; full collectors remain gated"),
        Some("stopped") => Some(
            "Flash quota probe stopped; review native technical evidence before resuming collectors",
        ),
        _ if dead >= 2 && bound.is_some() => Some(
            "Flash quota probe controller exited without clean admission; collectors remain gated",
        ),
        _ if bound.is_none()
            && now >= config["not_before_unix"].as_f64().unwrap_or(now) + 180.0 =>
        {
            Some("Flash quota probe has not published a live controller after scheduled reset")
        }
        _ => None,
    };
    if let Some(finding) = finding {
        event(
            state,
            &format!("{key}:terminal"),
            root,
            finding,
            json!({"phase":status["phase"],"automatic_resume":false}),
        );
    }
}
