//! Complete census first; optional expensive replay only for declared game IDs.
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context as _, ensure};
use baylee_engine::choice::PlayerAction;
use baylee_gamehost::record::{Line, Source};
use serde_json::{Value, json};

use super::{records, trace};

pub(super) fn run(root: &Path, expected: usize, traces: &[u64]) -> anyhow::Result<Value> {
    let mut seen = BTreeSet::new();
    let mut rows = Vec::new();
    let mut identities = Vec::new();
    let wanted: BTreeSet<_> = traces.iter().copied().collect();
    let mut totals = BTreeMap::<String, u64>::new();
    for directory in records::directories(root)? {
        let (index_sha, entries) = records::entries(&directory)?;
        identities.push(json!({"directory":directory,"games_index_sha256":index_sha}));
        for entry in entries {
            ensure!(seen.insert(entry.i), "duplicate game index {}", entry.i);
            let (digest, lines) = records::record(&directory, &entry)?;
            let counts = count(&lines, &entry.rollout_sidecar)
                .with_context(|| format!("game {}", entry.i))?;
            for (name, value) in &counts {
                *totals.entry(name.clone()).or_default() += value;
            }
            let replay = if wanted.contains(&entry.i) {
                Some(trace::replay(&lines).with_context(|| format!("replay game {}", entry.i))?)
            } else {
                None
            };
            rows.push(json!({"i":entry.i,"game":entry.game,"record_sha256":digest,
                "counts":counts,"trace":replay}));
        }
    }
    ensure!(
        seen.len() == expected,
        "found {} games, expected {expected}",
        seen.len()
    );
    ensure!(wanted.is_subset(&seen), "requested trace game was absent");
    rows.sort_by_key(|row| row["i"].as_u64());
    Ok(
        json!({"schema_version":1,"engine_revision":baylee_build::COMMIT,"engine_dirty":baylee_build::DIRTY,
        "expected_games":expected,"scope":"complete indexed action census; traces are hash-checked authorized-view replay, never policy training data or strength evidence",
        "games":rows,"game_count":seen.len(),"totals":totals,"inputs":identities}),
    )
}

fn count(lines: &[Line], sidecar: &[Value]) -> anyhow::Result<BTreeMap<String, u64>> {
    let mut result = BTreeMap::new();
    let mut neural_index = 0;
    let mut previous_cancel = false;
    let mut seat_cancel = [false; 8];
    for line in lines {
        let Line::Input {
            seat, by, action, ..
        } = line
        else {
            continue;
        };
        ensure!(usize::from(*seat) < seat_cancel.len(), "invalid seat");
        *result.entry("all_inputs".into()).or_default() += 1;
        let cancelled = matches!(action, PlayerAction::CancelCast);
        if cancelled {
            *result.entry("all_cancellations".into()).or_default() += 1;
            *result
                .entry(format!("cancel_record_source_{by:?}"))
                .or_default() += 1;
        }
        if *by == Source::Seat {
            let provenance =
                &sidecar.get(neural_index).context("missing seat sidecar")?["provenance"];
            let source = provenance["source"]
                .as_str()
                .context("missing behavior source")?;
            neural_index += 1;
            *result.entry(format!("seat_source_{source}")).or_default() += 1;
            if cancelled {
                *result
                    .entry(format!("cancel_policy_source_{source}"))
                    .or_default() += 1;
            }
            if source == "House" {
                if previous_cancel {
                    *result
                        .entry("fallback_after_global_cancel".into())
                        .or_default() += 1;
                }
                if seat_cancel[usize::from(*seat)] {
                    *result
                        .entry("fallback_after_same_seat_cancel".into())
                        .or_default() += 1;
                }
            }
        }
        previous_cancel = cancelled;
        seat_cancel[usize::from(*seat)] = cancelled;
    }
    ensure!(neural_index == sidecar.len(), "unused seat sidecars");
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input(n: u64, seat: u8, action: PlayerAction) -> Line {
        Line::Input {
            n,
            at: 0,
            seat,
            by: Source::Seat,
            action,
            hash: String::new(),
            seen: None,
        }
    }
    #[test]
    fn counts_global_and_same_seat_sequences_separately_and_rejects_misalignment() {
        let lines = vec![
            input(1, 0, PlayerAction::CancelCast),
            input(2, 1, PlayerAction::PassPriority),
            input(3, 0, PlayerAction::PassPriority),
        ];
        let sources =
            ["Net", "House", "House"].map(|source| json!({"provenance":{"source":source}}));
        let counts = count(&lines, &sources).unwrap();
        assert_eq!(counts["all_cancellations"], 1);
        assert_eq!(counts["cancel_policy_source_Net"], 1);
        assert_eq!(counts["fallback_after_global_cancel"], 1);
        assert_eq!(counts["fallback_after_same_seat_cancel"], 1);
        assert!(count(&lines, &sources[..2]).is_err());
        assert!(count(&lines[..2], &sources).is_err());
    }
}
