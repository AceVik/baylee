//! Cheap analytical model. No engine, random generator, shop model or player telemetry.
use anyhow::{Result, ensure};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(super) struct Report {
    pub scope: &'static str,
    pub note: &'static str,
    pub target_wins: u8,
    pub defeat_limit: u8,
    pub assumed_win_per_mille: u16,
    pub max_duels: u8,
    pub conquest_probability: f64,
    pub exhaustion_probability: f64,
    pub expected_duels: f64,
    pub stages: Vec<Stage>,
}

#[derive(Debug, Serialize)]
pub(super) struct Stage {
    pub ordinal: u8,
    pub probability_of_reaching: f64,
    /// Initial 18 + 12 per preceding duel, before any spending/resale.
    pub gross_gold_before_duel: u32,
}

pub(super) fn calculate(wins: u8, defeats: u8, win_per_mille: u16) -> Result<Report> {
    ensure!(
        (1..=32).contains(&wins) && (1..=32).contains(&defeats) && win_per_mille <= 1000,
        "pacing input outside bounded model"
    );
    let maximum = wins + defeats - 1;
    let mut report = Report {
        scope: "independent_win_probability_toy_model",
        note: "Hypothetical independent duel outcomes; not observed balance, acquisition chances or player skill.",
        target_wins: wins,
        defeat_limit: defeats,
        assumed_win_per_mille: win_per_mille,
        max_duels: maximum,
        conquest_probability: 0.0,
        exhaustion_probability: 0.0,
        expected_duels: 0.0,
        stages: (1..=maximum)
            .map(|ordinal| Stage {
                ordinal,
                probability_of_reaching: 0.0,
                gross_gold_before_duel: 18 + 12 * u32::from(ordinal - 1),
            })
            .collect(),
    };
    let wins = usize::from(wins);
    let defeats = usize::from(defeats);
    let p = f64::from(win_per_mille) / 1000.0;
    let mut states = vec![vec![0.0; defeats + 1]; wins + 1];
    states[0][0] = 1.0;
    for won in 0..=wins {
        for lost in 0..=defeats {
            let mass = states[won][lost];
            if won == wins {
                report.conquest_probability += mass;
            } else if lost == defeats {
                report.exhaustion_probability += mass;
            } else {
                report.stages[won + lost].probability_of_reaching += mass;
                report.expected_duels += mass;
                states[won + 1][lost] += mass * p;
                states[won][lost + 1] += mass * (1.0 - p);
            }
        }
    }
    Ok(report)
}
