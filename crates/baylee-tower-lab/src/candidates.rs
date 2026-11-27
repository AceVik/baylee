//! Calibrating the house ladder cell by cell: each stage's builds from `K`
//! independent sets of scripted runs, every candidate measured against the
//! same field (set 0's builds of every other starter at that stage), in both
//! chairs. The report suggests, per cell, the set whose build came closest
//! to even; that table is selection, so the ladder it makes is measured again
//! on fresh deals (`ladder`) before it counts. No build leaves the process.
use crate::{
    duel::{self, Bounds},
    jobs,
    tally::{Tally, round},
};
use baylee_tower::v2::{MAX_STAGES, package::Cohort};
use baylee_tower_content::{Content, Ladder};

/// Measure `candidates` (at least one set); `field` opponents per cell.
#[allow(clippy::too_many_arguments)] // the measurement's complete inputs
pub fn measure(
    content: &Content,
    cohort: &Cohort,
    candidates: &[Ladder],
    seeds: u32,
    field: usize,
    base: u64,
    threads: usize,
    stages: &[u8],
) -> serde_json::Value {
    let count = content.starters.len();
    let wanted = |stage: u8| stages.is_empty() || stages.contains(&stage);
    // (stage, archetype, candidate, opponent, seed, candidate in the first chair)
    let mut matches = Vec::new();
    for stage in (1..=MAX_STAGES).filter(|stage| wanted(*stage)) {
        for archetype in 0..count {
            for (candidate, _) in candidates.iter().enumerate() {
                for step in 0..field.min(count - 1) {
                    let opponent =
                        (archetype + 1 + step * (count - 1) / field.min(count - 1)) % count;
                    if opponent == archetype {
                        continue;
                    }
                    for deal in 0..seeds {
                        let seed = duel::mix(
                            base ^ 0x6361_6e64,
                            u64::from(stage) << 48
                                | (archetype as u64) << 32
                                | (opponent as u64) << 16
                                | u64::from(deal),
                        );
                        for first in [true, false] {
                            matches.push((stage, archetype, candidate, opponent, seed, first));
                        }
                    }
                }
            }
        }
    }
    let results = jobs::run(
        &matches,
        threads,
        |&(stage, archetype, candidate, opponent, seed, first)| {
            let index = usize::from(stage) - 1;
            let own = &candidates[candidate].stages[index][archetype];
            let other = &candidates[0].stages[index][opponent];
            let (a, b) = if first { (own, other) } else { (other, own) };
            duel::play(
                &content.catalog,
                cohort,
                stage,
                a,
                b,
                seed,
                Bounds::default(),
            )
        },
        |done, total| {
            if done % 1000 == 0 || done == total {
                eprintln!("candidates: {done}/{total} duels");
            }
        },
    );
    let stages_n = usize::from(MAX_STAGES);
    let mut cells = vec![vec![vec![Tally::default(); candidates.len()]; count]; stages_n];
    for (&(stage, archetype, candidate, _, _, first), played) in matches.iter().zip(&results) {
        cells[usize::from(stage) - 1][archetype][candidate].add(played, first);
    }
    let mut picks = vec![vec![0_u8; count]; stages_n];
    let mut report = Vec::new();
    for stage in (1..=MAX_STAGES).filter(|stage| wanted(*stage)) {
        let index = usize::from(stage) - 1;
        let mut rows = Vec::new();
        for archetype in 0..count {
            let scores: Vec<Option<f64>> =
                cells[index][archetype].iter().map(Tally::score).collect();
            let best = scores
                .iter()
                .enumerate()
                .filter_map(|(candidate, score)| score.map(|s| (candidate, (s - 0.5).abs())))
                .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
                .map_or(0, |(candidate, _)| candidate);
            picks[index][archetype] = u8::try_from(best).unwrap_or(0);
            rows.push(serde_json::json!({
                "starter": archetype + 1,
                "scores": scores.iter().map(|s| s.map(round)).collect::<Vec<_>>(),
                "pick": best,
            }));
        }
        report.push(serde_json::json!({"stage": stage, "cells": rows}));
    }
    serde_json::json!({
        "kind": "candidates",
        "sets": candidates.len(),
        "seeds": seeds,
        "field": field,
        "duels": matches.len(),
        "stages": report,
        "picks": picks,
    })
}
