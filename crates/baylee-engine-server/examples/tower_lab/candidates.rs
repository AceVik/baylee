//! Calibrating the house ladder cell by cell. A cell's candidates are its
//! build from set 0 of the scripted runs with 0..=`trims` of its most valuable
//! relics unequipped, and its untrimmed builds from the further sets; each is
//! measured against the same field (set 0's untrimmed builds of other starters
//! at that stage) in both chairs. The report suggests, per cell, the candidate
//! that came closest to even, as `PICKS` codes; that is selection, so the
//! ladder it makes is measured again on fresh deals (`ladder`) before it
//! counts. No build leaves the process.
use crate::{
    duel::{self, Bounds},
    jobs,
    tally::{Tally, round},
};
use baylee_tower::v2::{
    MAX_STAGES,
    package::{Build, Cohort},
};
use baylee_tower_content::{Content, HOUSE_TRIMS, Ladder, house_trimmed};

/// One planned duel.
struct Match {
    stage: u8,
    archetype: usize,
    candidate: usize,
    opponent: usize,
    seed: u64,
    first: bool,
}

/// The candidates' `PICKS` codes: trims of set 0, then the other sets.
fn codes(sets: usize, trims: u8) -> Vec<u8> {
    let mut codes: Vec<u8> = (0..=trims.min(HOUSE_TRIMS - 1)).collect();
    for set in 1..sets {
        codes.push(u8::try_from(set).unwrap_or(0) * HOUSE_TRIMS);
    }
    codes
}

/// Measure every cell's candidates against `field` opponents of its stage.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // one measurement, read top to bottom
pub fn measure(
    content: &Content,
    cohort: &Cohort,
    sets: &[Ladder],
    trims: u8,
    seeds: u32,
    field: usize,
    base: u64,
    threads: usize,
    stages: &[u8],
) -> serde_json::Value {
    let count = content.starters.len();
    let codes = codes(sets.len(), trims);
    let field = field.clamp(1, count - 1);
    let wanted: Vec<u8> = (1..=MAX_STAGES)
        .filter(|stage| stages.is_empty() || stages.contains(stage))
        .collect();
    let build = |stage: u8, archetype: usize, code: u8| -> Build {
        let set = &sets[usize::from(code / HOUSE_TRIMS)];
        house_trimmed(
            &set.stages[usize::from(stage) - 1][archetype],
            code % HOUSE_TRIMS,
        )
    };
    let mut matches = Vec::new();
    for &stage in &wanted {
        for archetype in 0..count {
            for candidate in 0..codes.len() {
                for step in 0..field {
                    let opponent = (archetype + 1 + step * (count - 1) / field) % count;
                    for deal in 0..seeds {
                        let seed = duel::mix(
                            base ^ 0x6361_6e64,
                            u64::from(stage) << 48
                                | (archetype as u64) << 32
                                | (opponent as u64) << 16
                                | u64::from(deal),
                        );
                        for first in [true, false] {
                            matches.push(Match {
                                stage,
                                archetype,
                                candidate,
                                opponent,
                                seed,
                                first,
                            });
                        }
                    }
                }
            }
        }
    }
    let results = jobs::run(
        &matches,
        threads,
        |m| {
            let own = build(m.stage, m.archetype, codes[m.candidate]);
            let other = &sets[0].stages[usize::from(m.stage) - 1][m.opponent];
            let (a, b) = if m.first {
                (&own, other)
            } else {
                (other, &own)
            };
            duel::play(
                &content.catalog,
                cohort,
                m.stage,
                a,
                b,
                m.seed,
                Bounds::default(),
            )
        },
        |done, total| {
            if done % 2000 == 0 || done == total {
                eprintln!("candidates: {done}/{total} duels");
            }
        },
    );
    let stages_n = usize::from(MAX_STAGES);
    let mut cells = vec![vec![vec![Tally::default(); codes.len()]; count]; stages_n];
    for (m, played) in matches.iter().zip(&results) {
        cells[usize::from(m.stage) - 1][m.archetype][m.candidate].add(played, m.first);
    }
    let mut picks = vec![vec![0_u8; count]; stages_n];
    let mut report = Vec::new();
    for &stage in &wanted {
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
            picks[index][archetype] = codes[best];
            rows.push(serde_json::json!({
                "starter": archetype + 1,
                "scores": scores.iter().map(|s| s.map(round)).collect::<Vec<_>>(),
                "pick": codes[best],
            }));
        }
        report.push(serde_json::json!({"stage": stage, "cells": rows}));
    }
    serde_json::json!({
        "kind": "candidates",
        "codes": codes,
        "seeds": seeds,
        "field": field,
        "duels": matches.len(),
        "stages": report,
        "picks": picks,
    })
}
