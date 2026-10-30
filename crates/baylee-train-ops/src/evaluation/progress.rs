//! Read-only experiment inspection and derived evaluation reports.
use super::{atomic_json, read_json, summarize};
use anyhow::Result;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
fn children(root: &Path) -> Vec<PathBuf> {
    let mut files = std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .collect::<Vec<_>>();
    files.sort();
    files
}
/// Inspect jobs and training logs, refreshing derived uncertainty when stale.
///
/// # Errors
/// Returns an error when a valid evaluation cannot be summarized or written.
pub fn inspect(root: &Path) -> Result<Value> {
    let mut jobs = Vec::new();
    for directory in children(&root.join("jobs")) {
        if let Ok(v) = read_json(&directory.join("job.json")) {
            jobs.push(json!({"stage":directory.file_name().unwrap_or_default().to_string_lossy(),"status":v["status"],"reason":v["reason"],"log":directory}));
        }
    }
    let mut latest = Value::Null;
    let mut validation = Value::Null;
    if let Ok(bytes) = std::fs::read(root.join("jobs/supervised/stdout.log")) {
        for line in String::from_utf8_lossy(&bytes).lines() {
            if line.starts_with("[train4] step=") {
                latest = json!(line);
            }
            if line.starts_with("[validation4]")
                && let Some(i) = line.find('{')
                && let Ok(v) = serde_json::from_str(&line[i..])
            {
                validation = v;
            }
        }
    }
    let mut evaluations = serde_json::Map::new();
    for path in children(&root.join("evaluation")) {
        if path.extension().is_none_or(|e| e != "json")
            || path.to_string_lossy().ends_with(".uncertainty.json")
        {
            continue;
        }
        let Ok(v) = read_json(&path) else {
            continue;
        };
        if v["no_baseline"] == true || v["deal_details"].as_array().is_none_or(Vec::is_empty) {
            continue;
        }
        let output = path.with_extension("uncertainty.json");
        let cached = read_json(&output).ok();
        let fresh = std::fs::metadata(&output)
            .and_then(|s| s.modified())
            .ok()
            .zip(std::fs::metadata(&path).and_then(|s| s.modified()).ok())
            .is_some_and(|(a, b)| a >= b);
        let summary = if fresh { cached } else { None };
        let summary = if let Some(s) = summary {
            s
        } else {
            let s = summarize(&v, 20000)?;
            atomic_json(&output, &s)?;
            s
        };
        evaluations.insert(
            path.file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            summary,
        );
    }
    let result = read_json(&root.join("result.json")).unwrap_or(Value::Null);
    let active = jobs
        .iter()
        .any(|j| j["status"] == "starting" || j["status"] == "running");
    let failed = jobs.iter().any(|j| j["status"] == "quarantined");
    Ok(
        json!({"root":root,"jobs":jobs,"latest_step":latest,"validation":validation,"evaluations":evaluations,"finished":!active && (!result.is_null() || failed),"result":result}),
    )
}
