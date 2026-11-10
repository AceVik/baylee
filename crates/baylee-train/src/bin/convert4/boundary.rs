//! Records written before the activator boundary stop the conversion.
//!
//! From `baylee_train::convert4::ACTIVATOR_BOUNDARY` on, the engine lets go
//! of the activator after a loyalty ability and names none for a special
//! action or a prepared cast; the field is in the snapshot hash, so a record
//! written before it replays on a later build with other hashes at exactly
//! those positions. The replay audit names such a game
//! (`BOUNDARY_REFUSAL`) instead of calling it a divergence. Leaving it out
//! would quietly take every game with a planeswalker out of a dataset, so
//! the run stops instead, after `rejected-games.json` lists them all, and
//! `--llm-bc-skip-unreplayable` cannot skip them either: such a record is
//! converted by the runtime that wrote it.

use anyhow::bail;
use baylee_train::convert4::{BOUNDARY_REFUSAL, BOUNDARY_RULE};
use serde_json::Value;

/// Refuses the run when any rejected game was written before the boundary.
pub(super) fn refuse_records_before_it(rejected: &[Value]) -> anyhow::Result<()> {
    let before: Vec<&Value> = rejected
        .iter()
        .filter(|game| {
            game["reason"]
                .as_str()
                .is_some_and(|reason| reason.starts_with(BOUNDARY_REFUSAL))
        })
        .collect();
    let Some(first) = before.first() else {
        return Ok(());
    };
    bail!(
        "{} of the selected games were written before the activator boundary \
         (first: game {} in {}: {}); {BOUNDARY_RULE}. rejected-games.json lists \
         them; no dataset was published",
        before.len(),
        first["game_id"],
        first["shard"],
        first["reason"].as_str().unwrap_or_default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_record_from_before_the_boundary_stops_the_run_and_names_the_rule() {
        let reason = format!(
            "{BOUNDARY_REFUSAL} written by 0.1.0+build.0 (1259551af4), converted by \
             0.1.0+build.0 (eb40dc593a): 3 of its engine hashes differ"
        );
        let rejected = vec![
            json!({"game_id": 4, "shard": "a/records.jsonl.gz", "reason": "unencoded decisions"}),
            json!({"game_id": 7, "shard": "a/records.jsonl.gz", "reason": reason}),
        ];
        let error = refuse_records_before_it(&rejected).unwrap_err().to_string();
        assert!(error.starts_with("1 of the selected games"), "{error}");
        assert!(error.contains("game 7"), "{error}");
        assert!(error.contains("1259551af4"), "{error}");
        assert!(error.contains(BOUNDARY_RULE), "{error}");
    }

    #[test]
    fn ordinary_rejections_leave_the_run_as_it_was() {
        let rejected = vec![
            json!({"game_id": 4, "reason": "unencoded decisions"}),
            json!({"game_id": 5, "reason": "replay audit: Diverged { n: Some(3) }"}),
        ];
        assert!(refuse_records_before_it(&rejected).is_ok());
        assert!(refuse_records_before_it(&[]).is_ok());
    }
}
