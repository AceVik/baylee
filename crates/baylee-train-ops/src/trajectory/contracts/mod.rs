//! Preserve converted BC teacher provenance before building or reopening caches.
mod llm;
mod shadow;
use crate::files;
use anyhow::{Context, Result, ensure};
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};
/// Reject BC-only labels before a policy-gradient caller touches a model or cache.
///
/// # Errors
/// Any BC marker or explicit false PPO eligibility is a hard refusal.
pub fn reject_policy_gradient(meta: &Value) -> Result<()> {
    ensure!(
        !marked(meta, "llm_bc_"),
        "Verified LLM labels are BC-only, never PPO behavior probabilities"
    );
    ensure!(
        !marked(meta, "shadow_bc_") && meta["ppo_eligible"] != false,
        "Shadow teacher labels are BC-only, never PPO behavior probabilities"
    );
    Ok(())
}
/// Validate both optional converted teacher contracts; ordinary datasets are unchanged.
///
/// # Errors
/// Rejects incomplete contracts, file tampering, escaped paths and contradictory labels.
pub fn validate(root: &Path, meta: &Value) -> Result<()> {
    shadow::validate(root, meta)?;
    llm::validate(root, meta)
}
fn marked(meta: &Value, prefix: &str) -> bool {
    meta.as_object()
        .is_some_and(|m| m.keys().any(|k| k.starts_with(prefix)))
}
fn digest(value: &Value) -> bool {
    value.as_str().is_some_and(|s| {
        s.len() == 64
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}
fn map<'a>(v: &'a Value, key: &str) -> Result<&'a Map<String, Value>> {
    v[key]
        .as_object()
        .with_context(|| format!("missing BC digest map: {key}"))
}
fn common(meta: &Value, prefix: &str, contract: &str) -> Result<()> {
    ensure!(
        meta[format!("{prefix}version")].as_u64() == Some(1)
            && meta[format!("{prefix}contract")] == contract
            && meta[format!("{prefix}status")] == "complete"
            && meta["ppo_eligible"] == false
            && meta["policy_provenance_version"].is_null()
            && matches!(meta["encoder_version"].as_u64(), Some(7 | 8))
            && meta["trajectory_version"].as_u64() == Some(2),
        "Incomplete or incompatible BC dataset contract"
    );
    Ok(())
}
fn bound(root: &Path, name: &str, expected: &Value) -> Result<PathBuf> {
    ensure!(digest(expected), "invalid BC file identity");
    let root = root.canonicalize()?;
    let path = root.join(name).canonicalize()?;
    ensure!(
        path.starts_with(&root) && path.is_file(),
        "BC file escapes dataset or is missing"
    );
    ensure!(
        files::sha256(&path)? == expected.as_str().context("missing BC digest")?,
        "BC companion or shard digest mismatch: {name}"
    );
    Ok(path)
}
fn small(path: &Path) -> Result<()> {
    ensure!(
        path.metadata()?.len() <= 64 * 1024 * 1024,
        "BC companion path/size invalid"
    );
    Ok(())
}
fn text<'a>(v: &'a Value, key: &str) -> Result<&'a str> {
    v[key]
        .as_str()
        .with_context(|| format!("missing BC string: {key}"))
}
fn positive(v: &Value, key: &str) -> Result<u64> {
    v[key]
        .as_u64()
        .filter(|n| *n > 0)
        .with_context(|| format!("invalid BC game identity: {key}"))
}
