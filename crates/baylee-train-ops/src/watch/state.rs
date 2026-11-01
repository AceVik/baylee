use super::{Config, files};
use anyhow::Result;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::Path};

/// Stable legacy JSON spacing for existing ASCII configuration event IDs.
pub(crate) fn canonical(value: &Value) -> Result<String> {
    Ok(match value {
        Value::Object(map) => {
            let ordered: BTreeMap<_, _> = map.iter().collect();
            let mut fields = Vec::new();
            for (key, value) in ordered {
                fields.push(format!(
                    "{}: {}",
                    serde_json::to_string(key)?,
                    canonical(value)?
                ));
            }
            format!("{{{}}}", fields.join(", "))
        }
        Value::Array(items) => format!(
            "[{}]",
            items
                .iter()
                .map(canonical)
                .collect::<Result<Vec<_>>>()?
                .join(", ")
        ),
        _ => serde_json::to_string(value)?,
    })
}

pub(super) fn event_id(config: &Value) -> Result<String> {
    Ok(files::hex(&Sha256::digest(canonical(config)?.as_bytes()))[..24].to_owned())
}

pub(super) fn finding(
    config: &Config,
    state: &Value,
    status: &Value,
    error: Option<&str>,
) -> Option<String> {
    let dead = state["dead_polls"].as_u64().unwrap_or(0) >= config.death_polls;
    let phase = status["phase"].as_str();
    if phase == Some("failed") {
        return Some("experiment failed".into());
    }
    if phase.is_some_and(|s| config.success_phases.iter().any(|p| p == s)) {
        let root = Path::new(&config.root);
        let missing: Vec<_> = config
            .artifacts
            .iter()
            .filter(|name| {
                let path = root.join(name);
                !path.is_file()
                    || path.extension().is_some_and(|s| s == "json")
                        && files::read_json(&path).is_err()
            })
            .collect();
        if missing.is_empty() {
            return Some("experiment completed; required artifacts exist".into());
        }
        return dead.then(|| {
            format!("incomplete terminal result; missing or invalid artifacts: {missing:?}")
        });
    }
    if state["status_errors"].as_u64().unwrap_or(0) >= config.error_polls {
        return Some(format!(
            "persistent unreadable status: {}",
            error.unwrap_or("unknown")
        ));
    }
    dead.then(|| "controller exited without a successful terminal result".into())
}
