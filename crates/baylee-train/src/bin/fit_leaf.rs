//! Fits the house search's leaf value (`baylee_ai::foresight`) on positions
//! from house-against-house games.
//!
//! `eval4 --leaf-log <file>` writes each house chair's features at the start
//! of every turn with the game's result. This reads those lines and fits a
//! logistic model without an intercept (the features are one side less the
//! other, so an even position is even) by projected gradient descent on
//! standardised features, and prints the weights as the constant
//! `baylee_ai::foresight::LEAF` holds, with the fit's log loss beside that
//! of calling every position even and that of the weights in the build.
//!
//! The loss is reported on whole games held out (`--holdout`): the positions
//! of one game share its result, so a game split between the two sets would
//! flatter the fit. `--only` and `--drop` choose the features fitted; the
//! others get no weight.
//!
//! `--pairs` fits on a probe log instead (`eval4 --foresight-probe`): the
//! leaves of search worlds that were played on to the end of the game. Two
//! answers tried at one question in one world whose games came out
//! differently make a pair, and the fit is on what the winner's leaf showed
//! over the loser's. What the two have in common (the decks, the deal, the
//! game so far) is the same on both sides and drops out, so the weights say
//! what a difference between two leaves of one search is worth, which is
//! what the search asks of them. Such pairs are few; `--ridge` keeps a fit
//! on few of them from running off.
//!
//! The games are the house's own, on the train split: the value says who is
//! ahead in a game between house players, never how to beat one of them.

#![allow(clippy::missing_docs_in_private_items)]

use std::collections::BTreeMap;
use std::io::{BufRead as _, BufReader};
use std::path::PathBuf;

use anyhow::Context as _;
use baylee_ai::foresight::{FEATURE_NAMES, FEATURES, LEAF};
use clap::Parser;

/// The features a weight below zero is left to: whose turn it is. More of
/// anything else counted is never worse for its side.
const UNBOUNDED: [&str; 1] = ["active"];

#[derive(Parser)]
struct Args {
    /// The leaf logs to fit on (with `--pairs`, the probe logs).
    #[arg(long, required = true, num_args = 1..)]
    log: Vec<PathBuf>,
    /// Leaves out positions before this turn.
    #[arg(long, default_value_t = 1)]
    from_turn: u32,
    /// Gradient steps.
    #[arg(long, default_value_t = 1500)]
    steps: usize,
    /// Every n-th game is held out to report the loss on; 0 holds none.
    #[arg(long, default_value_t = 5)]
    holdout: u64,
    /// Fits these features only (names as `FEATURE_NAMES`, comma-separated).
    #[arg(long, value_delimiter = ',')]
    only: Vec<String>,
    /// Leaves these features out.
    #[arg(long, value_delimiter = ',')]
    drop: Vec<String>,
    /// Lets every weight go below zero, to see what the bound costs.
    #[arg(long)]
    free: bool,
    /// Pulls the standardised weights towards zero by this much of their
    /// size a step.
    #[arg(long, default_value_t = 0.0)]
    ridge: f64,
    /// Fits on the pairs of a probe log that came out differently.
    #[arg(long)]
    pairs: bool,
    /// Threads the gradient is summed on.
    #[arg(long, default_value_t = 8)]
    threads: usize,
    /// Also writes the weights as plain numbers, one a line, for
    /// `foresight_probe --weights`.
    #[arg(long)]
    out: Option<PathBuf>,
}

/// A line of a leaf log, or of a probe log.
#[derive(serde::Deserialize)]
struct Row {
    /// The game the position is of; rows without one are each their own.
    #[serde(default)]
    g: Option<u64>,
    /// A probe log's question and world.
    #[serde(default)]
    r: u64,
    #[serde(default)]
    w: u64,
    /// A probe log's world that ended before its leaf: no position to read.
    #[serde(default)]
    t: bool,
    y: f64,
    turn: u32,
    f: Vec<f64>,
}

/// One position as the fit reads it: the result and the columns fitted.
struct Sample {
    y: f64,
    x: Vec<f64>,
}

fn sigmoid(z: f64) -> f64 {
    1.0 / (1.0 + (-z).exp())
}

fn dot(x: &[f64], w: &[f64]) -> f64 {
    x.iter().zip(w).map(|(x, w)| x * w).sum()
}

/// The log loss of calling a position of result `y` won with chance `p`.
fn log_loss(y: f64, p: f64) -> f64 {
    let p = p.clamp(1e-9, 1.0 - 1e-9);
    -(y * p.ln() + (1.0 - y) * (1.0 - p).ln())
}

fn loss(samples: &[Sample], w: &[f64]) -> f64 {
    let total: f64 = samples
        .iter()
        .map(|sample| log_loss(sample.y, sigmoid(dot(&sample.x, w))))
        .sum();
    #[allow(clippy::cast_precision_loss)]
    let mean = total / samples.len().max(1) as f64;
    mean
}

/// The gradient of the summed loss at `w`, summed over fixed chunks in
/// order, so the fit does not depend on how the threads are scheduled.
fn gradient(samples: &[Sample], w: &[f64], threads: usize) -> Vec<f64> {
    let chunk = samples.len().div_ceil(threads.max(1)).max(1);
    let parts: Vec<Vec<f64>> = std::thread::scope(|scope| {
        let handles: Vec<_> = samples
            .chunks(chunk)
            .map(|part| {
                scope.spawn(move || {
                    let mut sum = vec![0.0; w.len()];
                    for sample in part {
                        let error = sigmoid(dot(&sample.x, w)) - sample.y;
                        for (sum, x) in sum.iter_mut().zip(&sample.x) {
                            *sum += error * x;
                        }
                    }
                    sum
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("a gradient thread"))
            .collect()
    });
    let mut total = vec![0.0; w.len()];
    for part in parts {
        for (total, part) in total.iter_mut().zip(part) {
            *total += part;
        }
    }
    total
}

/// The largest eigenvalue of the samples' second-moment matrix, by power
/// iteration over every seventh sample: what bounds the step.
fn curvature(samples: &[Sample], columns: usize) -> f64 {
    let mut v = vec![1.0; columns];
    let mut value = 1.0;
    #[allow(clippy::cast_precision_loss)]
    let count = samples.len().div_ceil(7) as f64;
    for _ in 0..30 {
        let mut next = vec![0.0; columns];
        for sample in samples.iter().step_by(7) {
            let along = dot(&sample.x, &v);
            for (next, x) in next.iter_mut().zip(&sample.x) {
                *next += along * x;
            }
        }
        let norm = next.iter().map(|n| n * n).sum::<f64>().sqrt().max(1e-12);
        value = norm / count;
        v = next.into_iter().map(|n| n / norm).collect();
    }
    value
}

/// Reads the logs' positions from `from_turn` on, each log's games numbered
/// after those of the log before.
fn read(paths: &[PathBuf], from_turn: u32) -> anyhow::Result<Vec<Row>> {
    let mut rows: Vec<Row> = Vec::new();
    for path in paths {
        let file = std::fs::File::open(path).with_context(|| path.display().to_string())?;
        let offset = rows
            .iter()
            .filter_map(|row| row.g)
            .max()
            .map_or(0, |g| g + 1);
        for line in BufReader::new(file).lines() {
            let mut row: Row = serde_json::from_str(&line?)?;
            anyhow::ensure!(
                row.f.len() == FEATURES,
                "{}: {} features a position, this build reads {FEATURES}",
                path.display(),
                row.f.len()
            );
            row.g = row.g.map(|g| g + offset);
            if row.turn >= from_turn {
                rows.push(row);
            }
        }
    }
    anyhow::ensure!(!rows.is_empty(), "no positions");
    Ok(rows)
}

/// The pairs of a probe log: for every two leaves of one question and world
/// whose games came out differently, the first's columns less the second's,
/// and whether the first won. With each, its game and the difference of the
/// sums `built` gives the two.
fn pairs(
    rows: &[Row],
    sample: impl Fn(&Row) -> Sample,
    built: impl Fn(&Row) -> f64,
) -> Vec<(u64, Sample, f64)> {
    let mut worlds: BTreeMap<(u64, u64, u64), Vec<&Row>> = BTreeMap::new();
    for row in rows.iter().filter(|row| !row.t) {
        worlds
            .entry((row.g.unwrap_or(0), row.r, row.w))
            .or_default()
            .push(row);
    }
    let mut out = Vec::new();
    for ((game, _, _), leaves) in &worlds {
        for (at, a) in leaves.iter().enumerate() {
            for b in &leaves[at + 1..] {
                if (a.y - b.y).abs() < 0.25 {
                    continue;
                }
                let x = sample(a)
                    .x
                    .iter()
                    .zip(sample(b).x)
                    .map(|(a, b)| a - b)
                    .collect();
                let y = if a.y > b.y { 1.0 } else { 0.0 };
                out.push((*game, Sample { y, x }, built(a) - built(b)));
            }
        }
    }
    out
}

/// Accelerated projected gradient descent on the standardised `fit`,
/// restarted where it overshoots: the weights, none of `bounded` below zero.
fn descend(fit: &[Sample], bounded: &[bool], args: &Args) -> Vec<f64> {
    let columns = bounded.len();
    #[allow(clippy::cast_precision_loss)]
    let n = fit.len() as f64;
    let step = 1.0 / (0.25 * curvature(fit, columns) + args.ridge).max(1e-9);
    let mut w = vec![0.0; columns];
    let mut ahead = w.clone();
    let mut momentum = 1.0_f64;
    let mut last = f64::INFINITY;
    for at in 0..args.steps {
        let gradient = gradient(fit, &ahead, args.threads);
        let next: Vec<f64> = ahead
            .iter()
            .zip(&gradient)
            .zip(bounded)
            .map(|((w, g), bounded)| {
                let next = w - step * (g / n + args.ridge * w);
                if *bounded { next.max(0.0) } else { next }
            })
            .collect();
        if at % 25 == 0 {
            let now = loss(fit, &next);
            if now > last {
                momentum = 1.0;
                ahead.clone_from(&w);
                continue;
            }
            last = now;
        }
        let grown = f64::midpoint(1.0, 4.0f64.mul_add(momentum * momentum, 1.0).sqrt());
        let push = (momentum - 1.0) / grown;
        ahead = next
            .iter()
            .zip(&w)
            .map(|(next, w)| next + push * (next - w))
            .collect();
        w = next;
        momentum = grown;
    }
    w
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    for name in args.only.iter().chain(&args.drop) {
        anyhow::ensure!(
            FEATURE_NAMES.contains(&name.as_str()),
            "no feature named {name}"
        );
    }
    // The features fitted, by their place in `FEATURE_NAMES`.
    let used: Vec<usize> = (0..FEATURES)
        .filter(|&at| {
            let name = FEATURE_NAMES[at];
            (args.only.is_empty() || args.only.iter().any(|n| n == name))
                && !args.drop.iter().any(|n| n == name)
        })
        .collect();
    let rows = read(&args.log, args.from_turn)?;
    let sample = |row: &Row| Sample {
        y: row.y,
        x: used.iter().map(|&at| row.f[at]).collect(),
    };
    // The sum the weights in the build give a position.
    let built = |row: &Row| dot(&row.f, &LEAF);
    // Each sample with its game and the build's sum for it.
    let samples: Vec<(u64, Sample, f64)> = if args.pairs {
        pairs(&rows, sample, built)
    } else {
        rows.iter()
            .enumerate()
            .map(|(at, row)| (row.g.unwrap_or(at as u64), sample(row), built(row)))
            .collect()
    };
    anyhow::ensure!(!samples.is_empty(), "nothing to fit on");
    let (mut held, mut fit) = (Vec::new(), Vec::new());
    let mut built_loss = 0.0;
    for (game, sample, sum) in samples {
        if args.holdout > 0 && game % args.holdout == 0 {
            built_loss += log_loss(sample.y, sigmoid(sum));
            held.push(sample);
        } else {
            fit.push(sample);
        }
    }
    #[allow(clippy::cast_precision_loss)]
    let built_loss = built_loss / held.len().max(1) as f64;
    #[allow(clippy::cast_precision_loss)]
    let n = fit.len() as f64;
    let mut scale = vec![1.0; used.len()];
    for (at, scale) in scale.iter_mut().enumerate() {
        let square: f64 = fit.iter().map(|sample| sample.x[at] * sample.x[at]).sum();
        *scale = (square / n).sqrt().max(1e-9);
    }
    for sample in fit.iter_mut().chain(&mut held) {
        for (x, scale) in sample.x.iter_mut().zip(&scale) {
            *x /= scale;
        }
    }
    // More of anything counted is never worse for its side: a weight the
    // outcomes pull below zero is a correlation (a flooded hand has many
    // lands), not a reason to hold a land back.
    let bounded: Vec<bool> = used
        .iter()
        .map(|&at| !args.free && !UNBOUNDED.contains(&FEATURE_NAMES[at]))
        .collect();
    let w = descend(&fit, &bounded, &args);
    let mut leaf = [0.0; FEATURES];
    for (slot, &at) in used.iter().enumerate() {
        leaf[at] = w[slot] / scale[slot];
    }
    println!(
        "{}: {} fitted, {} held out; log loss held out {:.4} (even: {:.4}, the build's weights: {built_loss:.4}), fitted {:.4}",
        if args.pairs { "pairs" } else { "positions" },
        fit.len(),
        held.len(),
        loss(&held, &w),
        std::f64::consts::LN_2,
        loss(&fit, &w)
    );
    println!("pub const LEAF: [f64; FEATURES] = [");
    for (name, weight) in FEATURE_NAMES.iter().zip(leaf) {
        println!("    {weight:.5}, // {name}");
    }
    println!("];");
    if let Some(path) = &args.out {
        let text: Vec<String> = leaf.iter().map(|weight| format!("{weight:.5}")).collect();
        std::fs::write(path, text.join("\n") + "\n").with_context(|| path.display().to_string())?;
    }
    Ok(())
}
