//! Records written before an engine boundary stop the conversion.
//!
//! From `baylee_train::convert4::ACTIVATOR_BOUNDARY` on, the engine lets go
//! of the activator after a loyalty ability and names none for a special
//! action or a prepared cast; the field is in the snapshot hash, so a record
//! written before it replays on a later build with other hashes at exactly
//! those positions. From `QUESTION_BOUNDARY` on, it asks no order between
//! damage effects that cannot change the event and nothing once the game is
//! decided, so a record written before it holds answers a later build never
//! asks for. From `COUNT_BOUNDARY` on, "for each creature, its controller
//! sacrifices …" counts first, asks in APNAP order and sacrifices together,
//! so a record written before it parts from a later build where such an
//! instruction resolves. The replay audit names such a game
//! (`BOUNDARY_REFUSAL`, `QUESTION_REFUSAL`, `COUNT_REFUSAL`) instead of
//! calling it a divergence. Leaving it out would quietly take every game
//! with a planeswalker, with protection beside a mass damage spell, or with
//! a Fade Away, out of a dataset, so the run stops instead,
//! after `rejected-games.json` lists them all, and
//! `--llm-bc-skip-unreplayable` cannot skip them either: such a record is
//! converted by the runtime that wrote it.

use anyhow::bail;
use baylee_train::convert4::{
    BOUNDARY_REFUSAL, BOUNDARY_RULE, COUNT_REFUSAL, COUNT_RULE, QUESTION_REFUSAL, QUESTION_RULE,
};
use serde_json::Value;

/// The boundaries a rejected game may name: its reason's beginning, the
/// boundary's name and its rule.
const BOUNDARIES: [(&str, &str, &str); 3] = [
    (BOUNDARY_REFUSAL, "the activator boundary", BOUNDARY_RULE),
    (QUESTION_REFUSAL, "the question boundary", QUESTION_RULE),
    (COUNT_REFUSAL, "the count boundary", COUNT_RULE),
];

/// Refuses the run when any rejected game was written before a boundary.
pub(super) fn refuse_records_before_it(rejected: &[Value]) -> anyhow::Result<()> {
    for (refusal, name, rule) in BOUNDARIES {
        let before: Vec<&Value> = rejected
            .iter()
            .filter(|game| {
                game["reason"]
                    .as_str()
                    .is_some_and(|reason| reason.starts_with(refusal))
            })
            .collect();
        let Some(first) = before.first() else {
            continue;
        };
        bail!(
            "{} of the selected games were written before {name} \
             (first: game {} in {}: {}); {rule}. rejected-games.json lists \
             them; no dataset was published",
            before.len(),
            first["game_id"],
            first["shard"],
            first["reason"].as_str().unwrap_or_default(),
        )
    }
    Ok(())
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
    fn a_record_from_before_the_question_boundary_stops_the_run_and_names_the_rule() {
        let reason = format!(
            "{QUESTION_REFUSAL} written by 0.1.0+build.0 (5f5054b0b0), converted by \
             0.1.0+build.0 (c55a8c6eb0): from input 57, it answers 1 damage-order question(s)"
        );
        let rejected = vec![json!({"game_id": 9, "shard": "b/records.jsonl.gz", "reason": reason})];
        let error = refuse_records_before_it(&rejected).unwrap_err().to_string();
        assert!(error.contains("the question boundary"), "{error}");
        assert!(error.contains("game 9"), "{error}");
        assert!(error.contains(QUESTION_RULE), "{error}");
    }

    #[test]
    fn a_record_from_before_the_count_boundary_stops_the_run_and_names_the_rule() {
        let reason = format!(
            "{COUNT_REFUSAL} written by 0.1.0+build.0 (abf82af0ca), converted by \
             0.1.0+build.0 (445a4e2760): from input 258, it resolves \"for each creature, its \
             controller sacrifices …\" (Fade Away) creature by creature in battlefield order"
        );
        let rejected = vec![json!({"game_id": 3, "shard": "c/records.jsonl.gz", "reason": reason})];
        let error = refuse_records_before_it(&rejected).unwrap_err().to_string();
        assert!(error.contains("the count boundary"), "{error}");
        assert!(error.contains("game 3"), "{error}");
        assert!(error.contains(COUNT_RULE), "{error}");
    }

    #[test]
    fn ordinary_rejections_leave_the_run_as_it_was() {
        let rejected = vec![
            json!({"game_id": 4, "reason": "unencoded decisions"}),
            json!({"game_id": 5, "reason": "replay audit: Diverged { n: Some(3) }"}),
            json!({"game_id": 6, "reason": "seen replay audit: written before question \
                   boundary c55a8c6eb by 0.1.0+build.0 (5f5054b0b0): from input 3"}),
            json!({"game_id": 7, "reason": "seen replay audit: written before count \
                   boundary 445a4e276 by 0.1.0+build.0 (abf82af0ca): from input 258"}),
        ];
        assert!(refuse_records_before_it(&rejected).is_ok());
        assert!(refuse_records_before_it(&[]).is_ok());
    }
}
