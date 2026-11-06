//! The value heads on duels, against baselines a head has to beat.
//!
//! Reads `eval4 --value-log` lines and each game's result from the run's
//! summary. The constant baselines are fitted on the other split of the pair
//! (dev against fresh, fresh against dev), so nothing is scored on the data it
//! was fitted to, and the board baseline is a logistic regression on life, hand
//! and permanent differences, fitted the same way. Intervals are table
//! bootstraps, as everywhere else in the evaluation.
use super::summary::sample;
use anyhow::{Context, Result};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io::BufRead as _;
use std::path::Path;

/// One decision of the seat under test.
#[derive(Clone, Debug)]
pub struct Position {
    deal: u64,
    turn: u64,
    settled: bool,
    active: bool,
    decision_value: f64,
    observed_value: Option<f64>,
    features: [f64; 4],
    outcome: f64,
}

/// Which positions a row of the report counts.
#[derive(Clone, Copy, Debug)]
enum Slice {
    All,
    Settled,
    /// The first settled position of each turn: one position per own turn.
    SettledFirst,
    TheirTurn,
}

const BANDS: [(&str, u64, u64); 6] = [
    ("turns 1-4", 1, 4),
    ("turns 5-8", 5, 8),
    ("turns 9-12", 9, 12),
    ("turns 13-18", 13, 18),
    ("turns 19+", 19, u64::MAX),
    ("all turns", 0, u64::MAX),
];

/// Reads a run's positions: decisions the net itself answered, of games that
/// finished.
///
/// # Errors
/// Fails on an unreadable log or summary.
pub fn read_positions(log: &Path, summary: &Path) -> Result<Vec<Position>> {
    let summary: Value = serde_json::from_slice(&std::fs::read(summary)?)?;
    let mut outcomes = BTreeMap::new();
    for row in summary["deal_details"]
        .as_array()
        .context("missing deal_details")?
    {
        if row["net_capped"] != true {
            outcomes.insert(
                row["deal"].as_u64().context("missing deal")?,
                row["net_utility"].as_f64().context("missing utility")?,
            );
        }
    }
    let mut positions = Vec::new();
    for line in std::io::BufReader::new(std::fs::File::open(log)?).lines() {
        let row: Value = serde_json::from_str(&line?)?;
        let source = row["src"].as_str().unwrap_or("");
        if source.eq_ignore_ascii_case("forced") || row["fb"] == true {
            continue;
        }
        let deal = row["d"].as_u64().context("missing deal")?;
        let Some(&outcome) = outcomes.get(&deal) else {
            continue;
        };
        let number = |key: &str| row[key].as_f64().unwrap_or(0.0);
        positions.push(Position {
            deal,
            turn: row["turn"].as_u64().context("missing turn")?,
            settled: row["settled"] == true,
            active: row["active"] == true,
            decision_value: number("v"),
            observed_value: row["ov"].as_f64(),
            features: [
                1.0,
                (number("life") - number("opp_life")) / 10.0,
                (number("hand") - number("opp_hand")) / 5.0,
                (number("board") - number("opp_board")) / 5.0,
            ],
            outcome,
        });
    }
    Ok(positions)
}

fn selected(positions: &[Position], slice: Slice, band: (u64, u64)) -> Vec<&Position> {
    let mut seen = std::collections::BTreeSet::new();
    positions
        .iter()
        .filter(|p| (band.0..=band.1).contains(&p.turn))
        .filter(|p| match slice {
            Slice::All => true,
            Slice::Settled => p.settled,
            Slice::SettledFirst => p.settled && seen.insert((p.deal, p.turn)),
            Slice::TheirTurn => !p.active,
        })
        .collect()
}

/// A logistic regression on the board features, by Newton steps with a small ridge.
fn fit_board(positions: &[&Position]) -> [f64; 4] {
    let mut weights = [0.0_f64; 4];
    for _ in 0..12 {
        let mut gradient = [0.0_f64; 4];
        let mut hessian = [[0.0_f64; 4]; 4];
        for position in positions {
            let p = logistic(dot(&weights, &position.features));
            let spread = (p * (1.0 - p)).max(1e-6);
            for i in 0..4 {
                gradient[i] += (position.outcome - p) * position.features[i];
                for j in 0..4 {
                    hessian[i][j] += spread * position.features[i] * position.features[j];
                }
            }
        }
        for (i, row) in hessian.iter_mut().enumerate() {
            row[i] += 1e-3;
        }
        let step = solve(hessian, gradient);
        for (weight, change) in weights.iter_mut().zip(step) {
            *weight += change;
        }
    }
    weights
}

fn logistic(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

fn dot(a: &[f64; 4], b: &[f64; 4]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// Gaussian elimination with partial pivoting.
fn solve(mut matrix: [[f64; 4]; 4], mut rhs: [f64; 4]) -> [f64; 4] {
    for col in 0..4 {
        let pivot = (col..4)
            .max_by(|a, b| matrix[*a][col].abs().total_cmp(&matrix[*b][col].abs()))
            .unwrap_or(col);
        matrix.swap(col, pivot);
        rhs.swap(col, pivot);
        for row in col + 1..4 {
            let factor = matrix[row][col] / matrix[col][col];
            for k in col..4 {
                matrix[row][k] -= factor * matrix[col][k];
            }
            rhs[row] -= factor * rhs[col];
        }
    }
    let mut out = [0.0; 4];
    for row in (0..4).rev() {
        let tail: f64 = (row + 1..4).map(|k| matrix[row][k] * out[k]).sum();
        out[row] = (rhs[row] - tail) / matrix[row][row];
    }
    out
}

fn mean(values: impl Iterator<Item = f64>) -> f64 {
    let (mut sum, mut count) = (0.0, 0usize);
    for value in values {
        sum += value;
        count += 1;
    }
    sum / count.max(1) as f64
}

/// Area under the ROC curve of `scores` against outcomes above one half.
fn auc(scored: &mut [(f64, f64)]) -> Option<f64> {
    scored.sort_by(|a, b| a.0.total_cmp(&b.0));
    let (mut rank_sum, mut wins, mut i) = (0.0, 0usize, 0usize);
    while i < scored.len() {
        let mut j = i;
        while j < scored.len() && scored[j].0.to_bits() == scored[i].0.to_bits() {
            j += 1;
        }
        let rank = (i + j + 1) as f64 / 2.0;
        for item in &scored[i..j] {
            if item.1 > 0.5 {
                rank_sum += rank;
                wins += 1;
            }
        }
        i = j;
    }
    let losses = scored.len() - wins;
    (wins > 0 && losses > 0)
        .then(|| (rank_sum - (wins * (wins + 1)) as f64 / 2.0) / (wins as f64 * losses as f64))
}

/// The scale and shift of the logit that fit the outcomes best: a slope of one
/// and a shift of zero is a calibrated head.
fn recalibration(scored: &[(f64, f64)]) -> (f64, f64) {
    let (mut shift, mut slope) = (0.0_f64, 1.0_f64);
    for _ in 0..25 {
        let (mut g0, mut g1, mut h00, mut h01, mut h11) = (0.0, 0.0, 1e-6, 0.0, 1e-6);
        for (score, outcome) in scored {
            let x = (score.clamp(1e-4, 1.0 - 1e-4) / (1.0 - score.clamp(1e-4, 1.0 - 1e-4))).ln();
            let p = logistic(shift + slope * x);
            let spread = (p * (1.0 - p)).max(1e-6);
            g0 += outcome - p;
            g1 += (outcome - p) * x;
            h00 += spread;
            h01 += spread * x;
            h11 += spread * x * x;
        }
        let determinant = (h00 * h11 - h01 * h01).max(1e-9);
        shift += (h11 * g0 - h01 * g1) / determinant;
        slope += (h00 * g1 - h01 * g0) / determinant;
    }
    (slope, shift)
}

/// Ten bins of the score: its mean, the outcome's mean, the count.
fn bins(scored: &[(f64, f64)]) -> Vec<Value> {
    (0..10)
        .filter_map(|bin| {
            let inside: Vec<_> = scored
                .iter()
                .filter(|(score, _)| ((score * 10.0) as usize).min(9) == bin)
                .collect();
            (!inside.is_empty()).then(|| {
                json!({"bin": bin, "n": inside.len(),
                    "mean_value": mean(inside.iter().map(|s| s.0)),
                    "mean_outcome": mean(inside.iter().map(|s| s.1))})
            })
        })
        .collect()
}

/// Per table, the sum of squared-error gains of the head over a baseline, and
/// the count, for the bootstrap.
fn gain_interval(rows: &[(u64, f64)], resamples: usize) -> [f64; 2] {
    let mut tables: BTreeMap<u64, (f64, usize)> = BTreeMap::new();
    for (table, gain) in rows {
        let entry = tables.entry(*table).or_default();
        entry.0 += gain;
        entry.1 += 1;
    }
    let tables: Vec<_> = tables.into_values().collect();
    let mut rng = 20_261_106_u64;
    let mut means = Vec::with_capacity(resamples);
    for _ in 0..resamples {
        let (mut total, mut count) = (0.0, 0usize);
        for _ in &tables {
            let table = tables[sample(&mut rng, tables.len())];
            total += table.0;
            count += table.1;
        }
        means.push(total / count.max(1) as f64);
    }
    means.sort_by(f64::total_cmp);
    [
        means[(resamples as f64 * 0.025) as usize],
        means[(resamples as f64 * 0.975) as usize],
    ]
}

/// One row: `score` against the baselines, on `test`, baselines fitted on `fit`.
fn row(
    label: &str,
    test: &[&Position],
    fit: &[&Position],
    score: impl Fn(&Position) -> Option<f64>,
    resamples: usize,
) -> Value {
    let rate = mean(fit.iter().map(|p| p.outcome));
    let board = fit_board(fit);
    let scored: Vec<(f64, f64, u64, f64)> = test
        .iter()
        .filter_map(|p| {
            score(p).map(|s| (s, p.outcome, p.deal / 2, logistic(dot(&board, &p.features))))
        })
        .collect();
    if scored.is_empty() {
        return json!({"slice": label, "n": 0});
    }
    let brier = |f: &dyn Fn(&(f64, f64, u64, f64)) -> f64| {
        mean(scored.iter().map(|s| (f(s) - s.1).powi(2)))
    };
    let head = brier(&|s| s.0);
    let constant = brier(&|_| rate);
    let uniform = brier(&|_| 0.5);
    let board_brier = brier(&|s| s.3);
    let gains: Vec<(u64, f64)> = scored
        .iter()
        .map(|s| (s.2, (rate - s.1).powi(2) - (s.0 - s.1).powi(2)))
        .collect();
    let board_gains: Vec<(u64, f64)> = scored
        .iter()
        .map(|s| (s.2, (s.3 - s.1).powi(2) - (s.0 - s.1).powi(2)))
        .collect();
    let mut pairs: Vec<(f64, f64)> = scored.iter().map(|s| (s.0, s.1)).collect();
    let (slope, shift) = recalibration(&pairs);
    let calibration = bins(&pairs);
    let games = scored
        .iter()
        .map(|s| s.2)
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let head_auc = auc(&mut pairs);
    let mut board_pairs: Vec<(f64, f64)> = scored.iter().map(|s| (s.3, s.1)).collect();
    json!({
        "slice": label, "n": scored.len(), "tables": games,
        "outcome_rate": mean(scored.iter().map(|s| s.1)),
        "constant_rate_from_other_split": rate,
        "mean_value": mean(scored.iter().map(|s| s.0)),
        "brier": head, "brier_uniform": uniform, "brier_constant": constant,
        "brier_board": board_brier,
        "skill_vs_constant": 1.0 - head / constant,
        "gain_vs_constant": constant - head,
        "gain_vs_constant_95": gain_interval(&gains, resamples),
        "gain_vs_board": board_brier - head,
        "gain_vs_board_95": gain_interval(&board_gains, resamples),
        "auc": head_auc, "auc_board": auc(&mut board_pairs),
        "calibration_slope": slope, "calibration_shift": shift,
        "calibration": calibration,
    })
}

/// The report on `test`, its baselines fitted on `fit`, for both heads.
#[must_use]
pub fn report(test: &[Position], fit: &[Position], resamples: usize) -> Value {
    let mut rows = Vec::new();
    for (head, score) in [
        (
            "decision (K as run)",
            (|p: &Position| Some(p.decision_value)) as fn(&Position) -> Option<f64>,
        ),
        ("observation (K=0, the search's leaf)", |p| p.observed_value),
    ] {
        for (slice_name, slice) in [
            ("all decisions", Slice::All),
            ("settled", Slice::Settled),
            ("settled, first of the turn", Slice::SettledFirst),
            ("their turn", Slice::TheirTurn),
        ] {
            for (band, low, high) in BANDS {
                let on_test = selected(test, slice, (low, high));
                let on_fit = selected(fit, slice, (low, high));
                let label = format!("{head} / {slice_name} / {band}");
                rows.push(row(&label, &on_test, &on_fit, score, resamples));
            }
        }
    }
    Value::Array(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auc_of_a_perfect_score_is_one() {
        let mut scored = vec![(0.1, 0.0), (0.2, 0.0), (0.8, 1.0), (0.9, 1.0)];
        assert_eq!(auc(&mut scored), Some(1.0));
        let mut ties = vec![(0.5, 0.0), (0.5, 1.0)];
        assert_eq!(auc(&mut ties), Some(0.5));
    }

    #[test]
    fn a_calibrated_score_has_slope_one() {
        let mut scored = Vec::new();
        for (score, wins) in [(0.2, 2), (0.5, 5), (0.8, 8)] {
            for i in 0..10 {
                scored.push((score, f64::from(i < wins)));
            }
        }
        let (slope, shift) = recalibration(&scored);
        assert!((slope - 1.0).abs() < 1e-3, "{slope}");
        assert!(shift.abs() < 1e-3, "{shift}");
    }

    #[test]
    fn the_board_fit_finds_the_sign_of_life() {
        let positions: Vec<Position> = (0..200)
            .map(|i| {
                let ahead = i % 2 == 0;
                let noise = f64::from(i % 10 == 0);
                Position {
                    deal: i,
                    turn: 5,
                    settled: true,
                    active: true,
                    decision_value: 0.5,
                    observed_value: None,
                    features: [1.0, if ahead { 1.0 } else { -1.0 }, 0.0, 0.0],
                    outcome: if ahead { 1.0 - noise } else { noise },
                }
            })
            .collect();
        let refs: Vec<&Position> = positions.iter().collect();
        assert!(fit_board(&refs)[1] > 0.5);
    }
}
