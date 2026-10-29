//! Narrow repair of proven timestamp rounding, preserving all IDs/newer deliveries.
use crate::files;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::path::Path;
fn corrected(current: &Value, backup: &Value) -> Result<(Value, Vec<Value>)> {
    let mut updated = current.clone();
    let old = backup["delivered"]
        .as_object()
        .context("backup delivery ledger")?;
    let entries = updated["delivered"]
        .as_object_mut()
        .context("current delivery ledger")?;
    let mut changes = Vec::new();
    for (id, original) in old {
        let entry = entries
            .get_mut(id)
            .context("historical delivery ID missing; refuse replacement")?;
        let mut historical = original.clone();
        let mut present = entry.clone();
        let old_fields = historical
            .as_object_mut()
            .context("historical delivery object")?;
        let new_fields = present.as_object_mut().context("current delivery object")?;
        for field in ["time", "at"] {
            if let (Some(before), Some(after)) =
                (old_fields.remove(field), new_fields.remove(field))
            {
                if before != after {
                    let a = before.as_f64().context("historical timestamp number")?;
                    let b = after.as_f64().context("current timestamp number")?;
                    ensure!(
                        a.is_finite()
                            && b.is_finite()
                            && a >= 0.0
                            && b >= 0.0
                            && a.to_bits().abs_diff(b.to_bits()) <= 1,
                        "timestamp difference is not one ULP; operator review required"
                    );
                    changes.push(
                        json!({"event_id":id,"field":field,"before":after,"restored":before}),
                    );
                    entry[field] = before;
                }
            } else {
                ensure!(
                    original.get(field) == entry.get(field),
                    "timestamp fields differ"
                );
            }
        }
        ensure!(
            historical == present,
            "historical delivery payload differs; refuse replacement"
        );
    }
    Ok((updated, changes))
}
/// Audit or restore only proven one-ULP timestamp differences under the ledger lock.
/// # Errors
/// Rejects missing IDs, changed payloads, larger deltas, or existing output receipt.
pub fn run(state: &Path, backup: &Path, lock: &Path, out: &Path, apply: bool) -> Result<Value> {
    ensure!(!out.exists(), "preserving existing repair receipt");
    let _lock = files::lock(lock)?;
    let current = files::read_json(state)?;
    let original = files::read_json(backup)?;
    let before = files::sha256(state)?;
    let (updated, changes) = corrected(&current, &original)?;
    if apply && !changes.is_empty() {
        files::atomic_json(state, &updated)?;
        ensure!(
            files::read_json(state)? == updated,
            "lossless ledger verification failed"
        );
    }
    let receipt = json!({"schema_version":1,"kind":"historical-delivery-timestamp-roundtrip-repair","state":state,"backup":backup,"applied":apply,"changes":changes,"state_sha256_before":before,"state_sha256_after":files::sha256(state)?,"backup_sha256":files::sha256(backup)?,"historical_ids_preserved":true,"newer_entries_preserved":true});
    files::atomic_json(out, &receipt)?;
    Ok(receipt)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repairs_only_proven_rounding_preserving_new_delivery() -> Result<()> {
        let old: Value = serde_json::from_str(
            r#"{"delivered":{"old":{"at":1793240833.6509361,"receipt":"original"}}}"#,
        )?;
        let now: Value = serde_json::from_str(
            r#"{"delivered":{"old":{"at":1793240833.650936,"receipt":"original"},"new":{"at":1793290000.0,"receipt":"new"}},"next_check":123}"#,
        )?;
        let (fixed, changes) = corrected(&now, &old)?;
        ensure!(
            changes.len() == 1
                && fixed["delivered"]["old"] == old["delivered"]["old"]
                && fixed["delivered"]["new"] == now["delivered"]["new"]
                && fixed["next_check"] == 123
        );
        let mut wrong = now;
        wrong["delivered"]["old"]["receipt"] = json!("different");
        ensure!(corrected(&wrong, &old).is_err());
        Ok(())
    }
}
