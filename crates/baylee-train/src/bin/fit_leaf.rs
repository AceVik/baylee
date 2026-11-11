//! Fits the house search's leaf value (`baylee_ai::foresight`) on positions
//! from house-against-house games.
//!
//! `eval4 --leaf-log <file>` writes each house chair's features at the start
//! of every turn with the game's result. This reads those lines and fits a
//! logistic model without an intercept (the features are one side less the
//! other, so an even position is even) by gradient descent on standardised
//! features, and prints the weights as the constant
//! `baylee_ai::foresight::LEAF` holds, with the fit's log loss beside that
//! of calling every position even.
//!
//! The games are the house's own, on the train split: the value says who is
//! ahead in a game between house players, never how to beat one of them.

#![allow(clippy::missing_docs_in_private_items)]

use std::io::{BufRead as _, BufReader};
use std::path::PathBuf;

use anyhow::Context as _;
use baylee_ai::foresight::{FEATURE_NAMES, FEATURES};
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
    #[arg(long, default_value_t = 3000)]
    steps: usize,
    /// Every n-th game-position row is held out to report the loss on.
    #[arg(long, default_value_t = 5)]
    holdout: usize,
}

#[derive(serde::Deserialize)]
struct Row {
    y: f64,
    turn: u32,
    f: [f64; FEATURES],
}

fn loss(rows: &[&Row], weights: &[f64; FEATURES]) -> f64 {
    let total: f64 = rows
        .iter()
        .map(|row| {
            let z: f64 = row.f.iter().zip(weights).map(|(x, w)| x * w).sum();
            let p = (1.0 / (1.0 + (-z).exp())).clamp(1e-9, 1.0 - 1e-9);
            -(row.y * p.ln() + (1.0 - row.y) * (1.0 - p).ln())
        })
        .sum();
    #[allow(clippy::cast_precision_loss)]
    let mean = total / rows.len().max(1) as f64;
    mean
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let mut rows: Vec<Row> = Vec::new();
    for path in &args.log {
        let file = std::fs::File::open(path).with_context(|| path.display().to_string())?;
        for line in BufReader::new(file).lines() {
            let row: Row = serde_json::from_str(&line?)?;
            if row.turn >= args.from_turn {
                rows.push(row);
            }
        }
    }
    anyhow::ensure!(!rows.is_empty(), "no positions");
    let (held, fit): (Vec<_>, Vec<_>) = rows
        .iter()
        .enumerate()
        .partition(|(at, _)| args.holdout > 0 && at % args.holdout == 0);
    let held: Vec<&Row> = held.into_iter().map(|(_, row)| row).collect();
    let fit: Vec<&Row> = fit.into_iter().map(|(_, row)| row).collect();
    #[allow(clippy::cast_precision_loss)]
    let n = fit.len() as f64;
    let mut scale = [1.0; FEATURES];
    for (at, scale) in scale.iter_mut().enumerate() {
        let square: f64 = fit.iter().map(|row| row.f[at] * row.f[at]).sum();
        *scale = (square / n).sqrt().max(1e-9);
    }
    // Weights on the standardised features.
    let mut w = [0.0; FEATURES];
    for _ in 0..args.steps {
        let mut gradient = [0.0; FEATURES];
        for row in &fit {
            let z: f64 = (0..FEATURES).map(|at| row.f[at] / scale[at] * w[at]).sum();
            let p = 1.0 / (1.0 + (-z).exp());
            for at in 0..FEATURES {
                gradient[at] += (p - row.y) * row.f[at] / scale[at];
            }
        }
        for at in 0..FEATURES {
            w[at] -= 0.5 * gradient[at] / n;
            // More of anything counted is never worse for its side: a weight
            // the outcomes pull below zero is a correlation (a flooded hand
            // has many lands), not a reason to hold a land back.
            if FEATURE_NAMES[at] != "active" {
                w[at] = f64::max(w[at], 0.0);
            }
        }
    }
    let mut weights = [0.0; FEATURES];
    for at in 0..FEATURES {
        weights[at] = w[at] / scale[at];
    }
    println!(
        "positions: {} fitted, {} held out; log loss held out {:.4} (even: {:.4}), fitted {:.4}",
        fit.len(),
        held.len(),
        loss(&held, &weights),
        std::f64::consts::LN_2,
        loss(&fit, &weights)
    );
    println!("pub const LEAF: [f64; FEATURES] = [");
    for (name, weight) in FEATURE_NAMES.iter().zip(weights) {
        println!("    {weight:.5}, // {name}");
    }
    println!("];");
    Ok(())
}
