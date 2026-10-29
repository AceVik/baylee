//! Reviewed content-addressed staging for official anonymized gateway exports.
mod privacy;
mod strict_json;
#[cfg(test)]
mod tests;
mod validation;
use anyhow::{Context, Result, ensure};
use flate2::{Compression, GzBuilder, read::MultiGzDecoder};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
pub use validation::validate;
const SCHEMA: &str = "anonymized-live-records4-v1";
const CONSENT: &str = "Locally requested on 2026-10-25: human live games may be used for training.";
fn decompress(path: &Path) -> Result<Vec<u8>> {
    let mut raw = Vec::new();
    MultiGzDecoder::new(File::open(path)?).read_to_end(&mut raw)?;
    Ok(raw)
}
fn hash(raw: &[u8]) -> String {
    super::hex(&Sha256::digest(raw))
}
fn exclusions(path: Option<&Path>) -> Result<BTreeMap<String, String>> {
    let Some(path) = path else {
        return Ok(BTreeMap::new());
    };
    let text = std::fs::read_to_string(path)?;
    let values = if text.trim_start().starts_with('{') {
        serde_json::from_value(strict_json::parse(text.as_bytes())?)?
    } else {
        let mut values = BTreeMap::new();
        for line in text.lines() {
            let line = line.split('#').next().unwrap_or_default().trim();
            if line.is_empty() {
                continue;
            }
            let mut fields = line.splitn(2, char::is_whitespace);
            let digest = fields.next().context("missing exclusion digest")?;
            let reason = fields.next().map_or("known_smoke", str::trim);
            values.insert(digest.to_owned(), reason.to_owned());
        }
        values
    };
    for (digest, reason) in &values {
        ensure!(validation::hex(digest, 64), "invalid excluded SHA256");
        ensure!(
            !reason.is_empty()
                && reason.len() <= 80
                && reason
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_-".contains(&b)),
            "exclusion reasons must be anonymous lowercase labels"
        );
    }
    Ok(values)
}
fn outside_export(export: &Path, out: &Path) -> Result<()> {
    ensure!(
        !out.is_symlink(),
        "output must be outside the export directory"
    );
    let absolute = std::path::absolute(out)?;
    let mut existing = absolute.as_path();
    let mut tail = Vec::new();
    while !existing.exists() {
        tail.push(existing.file_name().context("invalid output")?.to_owned());
        existing = existing.parent().context("invalid output")?;
    }
    let mut resolved = existing.canonicalize()?;
    for component in tail.into_iter().rev() {
        resolved.push(component);
    }
    // Normalize parent components before testing containment.
    let mut normalized = PathBuf::new();
    for c in resolved.components() {
        if c == std::path::Component::ParentDir {
            normalized.pop();
        } else if c != std::path::Component::CurDir {
            normalized.push(c);
        }
    }
    ensure!(
        !normalized.starts_with(export),
        "output must be outside the export directory"
    );
    Ok(())
}

/// Validate an entire anonymous export before publishing immutable train-only records.
///
/// # Errors
/// Rejects unsafe paths, malformed records, private information and damaged stores.
pub fn import_records(
    export: &Path,
    out: &Path,
    neural_seat: u64,
    exclude: Option<&Path>,
) -> Result<Value> {
    ensure!(neural_seat < 2, "invalid neural seat");
    ensure!(
        export.is_dir() && !export.is_symlink(),
        "export directory required"
    );
    outside_export(&export.canonicalize()?, out)?;
    let mut files = std::fs::read_dir(export)?
        .map(|r| r.map(|e| e.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    files.sort();
    ensure!(
        !files.is_empty()
            && files.iter().all(|p| p.is_file()
                && !p.is_symlink()
                && p.file_name()
                    .and_then(|s| s.to_str())
                    .is_some_and(|s| s.len() == 15
                        && s.ends_with(".jsonl.gz")
                        && s.as_bytes()[..6].iter().all(u8::is_ascii_digit))),
        "expected only numbered official gateway export gzip records"
    );
    let excluded = exclusions(exclude)?;
    let mut incoming = BTreeMap::new();
    for path in files {
        let raw = decompress(&path)?;
        let metadata = validate(&raw, neural_seat)?;
        incoming.insert(hash(&raw), (path, metadata));
    }
    std::fs::create_dir_all(out)?;
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(out.join(".import.lock"))?;
    lock.lock()?;
    publish(out, &incoming, neural_seat, excluded)
}

fn publish(
    out: &Path,
    incoming: &BTreeMap<String, (PathBuf, Value)>,
    neural_seat: u64,
    excluded: BTreeMap<String, String>,
) -> Result<Value> {
    let manifest_path = out.join("manifest.json");
    let mut manifest = if manifest_path.exists() {
        let v = strict_json::parse(&std::fs::read(&manifest_path)?)?;
        ensure!(
            v["schema"] == SCHEMA && v["neural_seat"].as_u64() == Some(neural_seat),
            "incompatible existing import manifest"
        );
        v
    } else {
        json!({"schema":SCHEMA,"split":"train","neural_seat":neural_seat,"consent":CONSENT,"source":"official_anonymized_gateway_export","allowed_initial_uses":["outcomes","auxiliary","tactical_review"],"policy_targets":"none_until_review","ppo_eligible":false,"neural_policy":"greedy_without_behavior_logprob_not_for_imitation","human_policy":"not_assumed_good_teacher_needs_review","conversion_done":false,"training_done":false,"records":{},"excluded":{},"quarantined":{}})
    };
    for key in ["records", "excluded", "quarantined"] {
        ensure!(
            manifest[key].is_object(),
            "invalid existing import manifest"
        );
    }
    for (digest, reason) in excluded {
        manifest["excluded"][&digest] = json!(reason);
    }
    for (digest, (_, metadata)) in incoming {
        match metadata["human_role_status"].as_str() {
            Some("excluded_house") => {
                manifest["excluded"]
                    .as_object_mut()
                    .context("invalid exclusions")?
                    .entry(digest)
                    .or_insert(json!("house_opponent"));
            }
            Some("confirmed_human") => {}
            _ if manifest["excluded"].get(digest).is_none() => {
                manifest["quarantined"][digest] = json!("unknown_or_mixed_opponent_role");
            }
            _ => {}
        }
    }
    let excluded: Vec<_> = manifest["excluded"]
        .as_object()
        .context("invalid exclusions")?
        .keys()
        .cloned()
        .collect();
    for digest in excluded {
        manifest["quarantined"]
            .as_object_mut()
            .context("invalid quarantine")?
            .remove(&digest);
    }
    let records: Vec<_> = manifest["records"]
        .as_object()
        .context("invalid records")?
        .keys()
        .cloned()
        .collect();
    for digest in records {
        ensure!(
            validation::hex(&digest, 64),
            "invalid existing content digest"
        );
        let stored = out.join(format!("records/{digest}.jsonl.gz"));
        ensure!(
            hash(&decompress(&stored)?) == digest,
            "immutable stored record failed integrity"
        );
        if manifest["excluded"].get(&digest).is_some()
            || manifest["quarantined"].get(&digest).is_some()
        {
            manifest["records"]
                .as_object_mut()
                .context("invalid records")?
                .remove(&digest);
        }
    }
    let record_dir = out.join("records");
    std::fs::create_dir_all(&record_dir)?;
    for (digest, (source, metadata)) in incoming {
        if manifest["excluded"].get(digest).is_some()
            || manifest["quarantined"].get(digest).is_some()
        {
            continue;
        }
        let raw = decompress(source)?;
        ensure!(hash(&raw) == *digest, "export changed during import");
        let stored = record_dir.join(format!("{digest}.jsonl.gz"));
        if stored.exists() {
            ensure!(decompress(&stored)? == raw, "stored content conflict");
        } else {
            publish_record(&stored, &raw)?;
        }
        let mut entry = metadata.clone();
        entry["path"] = json!(format!("records/{digest}.jsonl.gz"));
        entry["split"] = json!("train");
        manifest["records"][digest] = entry;
    }
    super::atomic_json(&manifest_path, &manifest)?;
    Ok(
        json!({"records":manifest["records"].as_object().context("invalid records")?.len(),"excluded":manifest["excluded"].as_object().context("invalid exclusions")?.len(),"excluded_house":manifest["excluded"].as_object().context("invalid exclusions")?.values().filter(|v|**v=="house_opponent").count(),"quarantined":manifest["quarantined"].as_object().context("invalid quarantine")?.len(),"conversion_done":false,"training_done":false}),
    )
}
fn publish_record(path: &Path, raw: &[u8]) -> Result<()> {
    let temporary = path.with_extension(format!("{}.partial", uuid::Uuid::now_v7()));
    let result = (|| -> Result<()> {
        let file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)?;
        let mut encoder = GzBuilder::new()
            .mtime(0)
            .write(file, Compression::default());
        encoder.write_all(raw)?;
        let file = encoder.finish()?;
        file.sync_all()?;
        let mut permissions = file.metadata()?.permissions();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            permissions.set_mode(0o444);
        }
        #[cfg(not(unix))]
        permissions.set_readonly(true);
        file.set_permissions(permissions)?;
        std::fs::hard_link(&temporary, path)?;
        Ok(())
    })();
    let _ = std::fs::remove_file(temporary);
    result
}
