//! Fits the house search's leaf value (`baylee_ai::foresight`) on positions
//! from house-against-house games.
//!
//! `eval4 --leaf-log <file>` writes each house chair's features at the start
//! of every turn with the game's result. This reads those lines and fits a
//! logistic model without an intercept (the features are one side less the
//! other, so an even position is even) by projected gradient descent on
//! standardised features, and prints the weights as the constants
//! `baylee_ai::foresight::LEAF` and `LATE` hold, with the fit's log loss
//! beside that of calling every position even.
//!
//! The loss is reported on whole games held out (`--holdout`): positions of
//! one game share its result, so a game split between the two sets would
//! flatter the fit. `--only` and `--drop` choose the features fitted (the
//! others get no weight), `--late` also fits what each weight gains as the
//! game grows late (`baylee_ai::foresight::late_share`).
//!
//! The games are the house's own, on the train split: the value says who is
//! ahead in a game between house players, never how to beat one of them.

#![allow(clippy::missing_docs_in_private_items)]

use std::collections::BTreeMap;
use std::io::{BufRead as _, BufReader};
use std::path::PathBuf;

use anyhow::Context as _;
use baylee_ai::foresight::{FEATURE_NAMES, FEATURES, LATE, LEAF, MAY_BE_NEGATIVE, late_share};
use clap::Parser;

#[derive(Parser)]
struct Args {
    /// The leaf logs to fit on.
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
    /// Also fits each weight's change with the turn.
    #[arg(long)]
    late: bool,
    /// Lets every weight go below zero.
    #[arg(long)]
    free: bool,
    /// Pulls the standardised weights towards zero by this much of their
    /// size a position: what keeps a fit on few positions from running off.
    #[arg(long, default_value_t = 0.0)]
    ridge: f64,
    /// Threads the gradient is summed on.
    #[arg(long, default_value_t = 8)]
    threads: usize,
    /// Reads the logs as leaves of search worlds played to their end, and
    /// fits on the pairs of one root and world that ended differently: what
    /// one answer's leaf had over the other's, and which of the two won.
    #[arg(long)]
    pairs: bool,
    /// Also writes the weights as plain numbers, `LEAF` then `LATE`.
    #[arg(long)]
    out: Option<PathBuf>,
}

#[derive(serde::Deserialize)]
struct Row {
    /// The game the position is of; rows without one are each their own.
    #[serde(default)]
    g: Option<u64>,
    /// The root and world a leaf played to its end is of.
    #[serde(default)]
    p: Option<u64>,
    /// The world ended before its leaf: there is no position to read.
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
/// iteration: what bounds the step.
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
/// after the one before.
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

#[allow(clippy::too_many_lines)]
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
    let k = used.len();
    let columns = if args.late { 2 * k } else { k };
    let rows = read(&args.log, args.from_turn)?;
    let sample = |row: &Row| {
        let mut x: Vec<f64> = used.iter().map(|&at| row.f[at]).collect();
        if args.late {
            let late = late_share(row.turn);
            x.extend(used.iter().map(|&at| row.f[at] * late));
        }
        Sample { y: row.y, x }
    };
    // The sum the constants in the build give a position.
    let built_sum = |row: &Row| -> f64 {
        let late = late_share(row.turn);
        (0..FEATURES)
            .map(|at| row.f[at] * (LEAF[at] + late * LATE[at]))
            .sum()
    };
    // Each sample with its game (or root) and the build's sum for it.
    let mut samples: Vec<(u64, Sample, f64)> = Vec::new();
    if args.pairs {
        let mut groups: BTreeMap<u64, Vec<&Row>> = BTreeMap::new();
        for row in rows.iter().filter(|row| !row.t) {
            let root = row.p.context("a row without a root and world")?;
            groups.entry(root).or_default().push(row);
        }
        for (root, leaves) in &groups {
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
                    samples.push((*root, Sample { y, x }, built_sum(a) - built_sum(b)));
                }
            }
        }
    } else {
        for (at, row) in rows.iter().enumerate() {
            samples.push((row.g.unwrap_or(at as u64), sample(row), built_sum(row)));
        }
    }
    anyhow::ensure!(!samples.is_empty(), "nothing to fit on");
    let (mut held, mut fit) = (Vec::new(), Vec::new());
    // What the constants in the build say of the held-out positions.
    let mut built = 0.0;
    for (game, sample, sum) in samples {
        if args.holdout > 0 && game % args.holdout == 0 {
            built += log_loss(sample.y, sigmoid(sum));
            held.push(sample);
        } else {
            fit.push(sample);
        }
    }
    #[allow(clippy::cast_precision_loss)]
    let built = built / held.len().max(1) as f64;
    #[allow(clippy::cast_precision_loss)]
    let n = fit.len() as f64;
    let mut scale = vec![1.0; columns];
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
    // lands), not a reason to hold a land back. The features of
    // `MAY_BE_NEGATIVE` are the ones that rule is not true of. With late
    // weights the rule holds at both ends: at the start (the weight) and
    // late (the weight and its late part together).
    let bounded: Vec<bool> = used
        .iter()
        .map(|&at| !args.free && !MAY_BE_NEGATIVE.contains(&FEATURE_NAMES[at]))
        .collect();
    let project = |w: &mut [f64]| {
        for at in 0..k {
            if !bounded[at] {
                continue;
            }
            w[at] = f64::max(w[at], 0.0);
            if args.late {
                let floor = -w[at] / scale[at] * scale[k + at];
                w[k + at] = f64::max(w[k + at], floor);
            }
        }
    };
    let step = 1.0 / (0.25 * curvature(&fit, columns)).max(1e-9);
    // Accelerated projected gradient descent, restarted where it overshoots.
    let mut w = vec![0.0; columns];
    let mut ahead = w.clone();
    let mut momentum = 1.0_f64;
    let mut last = f64::INFINITY;
    for at in 0..args.steps {
        let gradient = gradient(&fit, &ahead, args.threads);
        let mut next: Vec<f64> = ahead
            .iter()
            .zip(&gradient)
            .map(|(w, g)| w - step * (g / n + args.ridge * w))
            .collect();
        project(&mut next);
        if at % 25 == 0 {
            let now = loss(&fit, &next);
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
    let mut leaf = [0.0; FEATURES];
    let mut grows = [0.0; FEATURES];
    for (slot, &at) in used.iter().enumerate() {
        leaf[at] = w[slot] / scale[slot];
        if args.late {
            grows[at] = w[k + slot] / scale[k + slot];
        }
    }
    println!(
        "positions: {} fitted, {} held out; log loss held out {:.4} (even: {:.4}, the build's weights: {built:.4}), fitted {:.4}",
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
    if args.late {
        println!("pub const LATE: [f64; FEATURES] = [");
        for (name, weight) in FEATURE_NAMES.iter().zip(grows) {
            println!("    {weight:.5}, // {name}");
        }
        println!("];");
    }
    if let Some(path) = &args.out {
        let text: Vec<String> = leaf
            .iter()
            .chain(&grows)
            .map(|weight| format!("{weight:.5}"))
            .collect();
        std::fs::write(path, text.join("\n") + "\n").with_context(|| path.display().to_string())?;
    }
    Ok(())
}
