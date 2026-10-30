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
    // Check teacher provenance (including LLM shard digests) before parsing
    // tensor payloads, also when reopening an existing cache after a change.
    shadow::validate(root, meta)?;
    llm::validate(root, meta)?;
    validate_encoder_rows(root, meta)
}
/// Check E9 metadata against actual JSON or packed tensor widths.
/// # Errors
/// Refuses absent/wrong E9 widths and legacy relabeling.
pub fn validate_encoder_rows(root: &Path, meta: &Value) -> Result<()> {
    let encoder = meta["encoder_version"].as_u64();
    if encoder == Some(9) {
        ensure!(
            meta["global_feat_width"] == 82,
            "Encoder9 metadata requires global_feat_width82"
        );
    } else if let Some(width) = meta.get("global_feat_width") {
        ensure!(width == 80, "Legacy encoder global width must remain80");
    }
    if !matches!(encoder, Some(7..=9)) {
        return Ok(());
    }
    for split in ["train", "val"] {
        if let Some(shards) = meta[format!("{split}_shards")].as_array() {
            for shard in shards {
                let name = shard.as_str().context("invalid E9 shard")?;
                ensure!(
                    Path::new(name).file_name().is_some_and(|n| n == name),
                    "invalid E9 shard path"
                );
                super::read_shard(&root.join(split).join(name), |row| {
                    let width = row
                        .get("global_feat_width")
                        .and_then(Value::as_u64)
                        .or_else(|| {
                            row["state"]["global_feat"]
                                .as_array()
                                .map(|a| a.len() as u64)
                        });
                    ensure!(
                        if encoder == Some(9) {
                            width == Some(82)
                        } else {
                            width != Some(82)
                        },
                        "Encoder shard tensor width differs from metadata"
                    );
                    Ok(())
                })?;
            }
        }
    }
    Ok(())
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
            && matches!(meta["encoder_version"].as_u64(), Some(7..=9))
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
