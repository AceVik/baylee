//! Probes how much a tree scorer could add to the house search's linear
//! leaf value: boosted depth-2 trees fitted on the residual of a linear fit
//! (`fit_leaf --out`), with the held-out log loss as trees are added.
//!
//! The trees read the leaf's features and the turn's late share. The value
//! is made antisymmetric as the linear one is: half of the trees' sum at
//! the position less their sum at the position seen from the other side.

#![allow(clippy::missing_docs_in_private_items)]

use std::io::{BufRead as _, BufReader};
use std::path::PathBuf;

use anyhow::Context as _;
use baylee_ai::foresight::{FEATURE_NAMES, FEATURES, late_share};
use clap::Parser;

#[derive(Parser)]
struct Args {
    /// The leaf logs to fit on.
    #[arg(long, required = true, num_args = 1..)]
    log: Vec<PathBuf>,
    /// The linear weights the trees are fitted on top of (`fit_leaf --out`).
    #[arg(long)]
    linear: PathBuf,
    /// Trees.
    #[arg(long, default_value_t = 100)]
    trees: usize,
    /// How much of each tree is added.
    #[arg(long, default_value_t = 0.1)]
    rate: f64,
    /// The least positions a leaf of a tree holds.
    #[arg(long, default_value_t = 400)]
    least: usize,
    /// Every n-th game is held out to report the loss on.
    #[arg(long, default_value_t = 5)]
    holdout: u64,
    /// Writes the model: the rate, the linear weights, then a tree a line.
    #[arg(long)]
    out: Option<PathBuf>,
}

#[derive(serde::Deserialize)]
struct Row {
    #[serde(default)]
    g: Option<u64>,
    y: f64,
    turn: u32,
    f: Vec<f64>,
}

const COLUMNS: usize = FEATURES + 1;

struct Sample {
    y: f64,
    /// The features and the late share.
    x: [f64; COLUMNS],
    /// The linear value's sum.
    base: f64,
}

#[derive(Clone, Copy, Debug)]
struct Split {
    column: usize,
    /// Left where the value is at most this.
    at: f64,
}

#[derive(Clone, Debug)]
struct Tree {
    root: Split,
    children: [Option<Split>; 2],
    /// Left-left, left-right, right-left, right-right.
    values: [f64; 4],
}

impl Tree {
    fn leaf(&self, x: &[f64; COLUMNS]) -> usize {
        let side = usize::from(x[self.root.column] > self.root.at);
        let below = self.children[side].map_or(0, |split| usize::from(x[split.column] > split.at));
        2 * side + below
    }
}

fn sigmoid(z: f64) -> f64 {
    1.0 / (1.0 + (-z).exp())
}

fn mirrored(x: &[f64; COLUMNS]) -> [f64; COLUMNS] {
    let mut other = *x;
    for value in &mut other[..FEATURES] {
        *value = -*value;
    }
    other
}

fn sum(trees: &[Tree], rate: f64, sample: &Sample) -> f64 {
    let other = mirrored(&sample.x);
    let grown: f64 = trees
        .iter()
        .map(|tree| tree.values[tree.leaf(&sample.x)] - tree.values[tree.leaf(&other)])
        .sum();
    sample.base + 0.5 * rate * grown
}

fn loss(samples: &[Sample], trees: &[Tree], rate: f64) -> f64 {
    let total: f64 = samples
        .iter()
        .map(|sample| {
            let p = sigmoid(sum(trees, rate, sample)).clamp(1e-9, 1.0 - 1e-9);
            -(sample.y * p.ln() + (1.0 - sample.y) * (1.0 - p).ln())
        })
        .sum();
    #[allow(clippy::cast_precision_loss)]
    let mean = total / samples.len().max(1) as f64;
    mean
}

/// The best split of `members` by gain, among `cuts`.
fn best_split(
    samples: &[Sample],
    members: &[usize],
    gradient: &[f64],
    curve: &[f64],
    cuts: &[Vec<f64>],
    least: usize,
) -> Option<(Split, f64)> {
    let total_g: f64 = members.iter().map(|&at| gradient[at]).sum();
    let total_h: f64 = members.iter().map(|&at| curve[at]).sum();
    let score = |g: f64, h: f64| g * g / (h + 1.0);
    let mut best: Option<(Split, f64)> = None;
    for (column, cuts) in cuts.iter().enumerate() {
        if cuts.is_empty() {
            continue;
        }
        // Per cut: what lies at or below it and above the cut before.
        let mut bins = vec![(0.0, 0.0, 0_usize); cuts.len() + 1];
        for &at in members {
            let bin = cuts.partition_point(|cut| *cut < samples[at].x[column]);
            bins[bin].0 += gradient[at];
            bins[bin].1 += curve[at];
            bins[bin].2 += 1;
        }
        let (mut g, mut h, mut n) = (0.0, 0.0, 0);
        for (bin, cut) in bins.iter().zip(cuts) {
            g += bin.0;
            h += bin.1;
            n += bin.2;
            if n < least || members.len() - n < least {
                continue;
            }
            let gain = score(g, h) + score(total_g - g, total_h - h) - score(total_g, total_h);
            if best.is_none_or(|(_, most)| gain > most) {
                best = Some((Split { column, at: *cut }, gain));
            }
        }
    }
    best.filter(|(_, gain)| *gain > 1e-9)
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let weights: Vec<f64> = std::fs::read_to_string(&args.linear)?
        .split_whitespace()
        .map(str::parse)
        .collect::<Result<_, _>>()?;
    anyhow::ensure!(weights.len() == 2 * FEATURES, "the linear weights");
    let (mut held, mut fit) = (Vec::new(), Vec::new());
    let mut offset = 0;
    for path in &args.log {
        let file = std::fs::File::open(path).with_context(|| path.display().to_string())?;
        let mut most = 0;
        for (line_at, line) in BufReader::new(file).lines().enumerate() {
            let row: Row = serde_json::from_str(&line?)?;
            anyhow::ensure!(row.f.len() == FEATURES, "features");
            let late = late_share(row.turn);
            let mut x = [0.0; COLUMNS];
            x[..FEATURES].copy_from_slice(&row.f);
            x[FEATURES] = late;
            let base: f64 = (0..FEATURES)
                .map(|at| row.f[at] * (weights[at] + late * weights[FEATURES + at]))
                .sum();
            let game = row.g.unwrap_or(line_at as u64);
            most = most.max(game + 1);
            let sample = Sample { y: row.y, x, base };
            if (game + offset) % args.holdout == 0 {
                held.push(sample);
            } else {
                fit.push(sample);
            }
        }
        offset += most;
    }
    // The cuts of a column: between its values, at most 48 of them.
    let cuts: Vec<Vec<f64>> = (0..COLUMNS)
        .map(|column| {
            let mut values: Vec<f64> = fit.iter().map(|sample| sample.x[column]).collect();
            values.sort_by(f64::total_cmp);
            values.dedup();
            let step = values.len().div_ceil(48).max(1);
            values
                .windows(2)
                .step_by(step)
                .map(|pair| f64::midpoint(pair[0], pair[1]))
                .collect()
        })
        .collect();
    let name = |column: usize| FEATURE_NAMES.get(column).copied().unwrap_or("late");
    let mut trees: Vec<Tree> = Vec::new();
    println!(
        "0 trees: held out {:.4}, fitted {:.4}",
        loss(&held, &trees, args.rate),
        loss(&fit, &trees, args.rate)
    );
    let everyone: Vec<usize> = (0..fit.len()).collect();
    for round in 1..=args.trees {
        let (gradient, curve): (Vec<f64>, Vec<f64>) = fit
            .iter()
            .map(|sample| {
                let p = sigmoid(sum(&trees, args.rate, sample));
                (p - sample.y, (p * (1.0 - p)).max(1e-6))
            })
            .unzip();
        let Some((root, _)) = best_split(&fit, &everyone, &gradient, &curve, &cuts, args.least)
        else {
            break;
        };
        let (left, right): (Vec<usize>, Vec<usize>) = everyone
            .iter()
            .partition(|&&at| fit[at].x[root.column] <= root.at);
        let mut tree = Tree {
            root,
            children: [None, None],
            values: [0.0; 4],
        };
        for (side, members) in [left, right].iter().enumerate() {
            let split = best_split(&fit, members, &gradient, &curve, &cuts, args.least);
            tree.children[side] = split.map(|(split, _)| split);
            let mut sums = [(0.0, 0.0); 2];
            for &at in members {
                let below = tree.children[side]
                    .map_or(0, |split| usize::from(fit[at].x[split.column] > split.at));
                sums[below].0 += gradient[at];
                sums[below].1 += curve[at];
            }
            for (below, (g, h)) in sums.iter().enumerate() {
                tree.values[2 * side + below] = -g / (h + 1.0);
            }
            if tree.children[side].is_none() {
                tree.values[2 * side + 1] = tree.values[2 * side];
            }
        }
        if round <= 12 {
            println!(
                "  tree {round}: {} <= {:.2}; then {:?} / {:?}; {:?}",
                name(tree.root.column),
                tree.root.at,
                tree.children[0].map(|s| (name(s.column), s.at)),
                tree.children[1].map(|s| (name(s.column), s.at)),
                tree.values
            );
        }
        trees.push(tree);
        if round % 20 == 0 || round == args.trees {
            println!(
                "{round} trees: held out {:.4}, fitted {:.4}",
                loss(&held, &trees, args.rate),
                loss(&fit, &trees, args.rate)
            );
        }
    }
    if let Some(path) = &args.out {
        let mut text = format!("{}\n", args.rate);
        let numbers: Vec<String> = weights.iter().map(f64::to_string).collect();
        text += &(numbers.join(" ") + "\n");
        for tree in &trees {
            let child = |split: Option<Split>| {
                split.map_or_else(|| "-1 0".to_owned(), |s| format!("{} {}", s.column, s.at))
            };
            text += &format!(
                "{} {} {} {} {} {} {} {}\n",
                tree.root.column,
                tree.root.at,
                child(tree.children[0]),
                child(tree.children[1]),
                tree.values[0],
                tree.values[1],
                tree.values[2],
                tree.values[3]
            );
        }
        std::fs::write(path, text)?;
    }
    Ok(())
}
