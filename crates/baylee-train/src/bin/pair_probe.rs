//! SCAFFOLD (not committed): what a leaf value's choices are worth by the
//! games played on from them.
//!
//! Reads a pair log (the leaves of sampled search roots, each world played
//! to its end) and reports, per root, how much more often the answer a leaf
//! value picks wins than the level's own answer does, beside the same for a
//! choice made from the played-out results of half the worlds and judged on
//! the other half.

#![allow(clippy::missing_docs_in_private_items)]

use std::collections::BTreeMap;
use std::io::{BufRead as _, BufReader};
use std::path::PathBuf;

use anyhow::Context as _;
use baylee_ai::foresight::{FEATURES, late_share};
use clap::Parser;

#[derive(Parser)]
struct Args {
    #[arg(long, required = true, num_args = 1..)]
    log: Vec<PathBuf>,
    /// Weights to value the leaves with (`fit_leaf --out`), each a column of
    /// the report; the logged value is always the first.
    #[arg(long, num_args = 0..)]
    weights: Vec<PathBuf>,
    /// The margin a challenger has to show.
    #[arg(long, default_value_t = 0.02)]
    margin: f64,
    /// Tree models to value the leaves with (`fit_leaf_trees --out`).
    #[arg(long, num_args = 0..)]
    trees: Vec<PathBuf>,
}

#[derive(serde::Deserialize)]
struct Row {
    r: u64,
    w: u64,
    c: usize,
    k: usize,
    t: bool,
    y: f64,
    turn: u32,
    v: f64,
    f: Vec<f64>,
}

/// One world of one candidate: the result played out and the leaf's value
/// under each set of weights.
#[derive(Clone)]
struct Leaf {
    t: bool,
    y: f64,
    values: Vec<f64>,
}

/// A root's candidates, each by world.
struct Root {
    kind: usize,
    candidates: BTreeMap<usize, BTreeMap<u64, Leaf>>,
}

/// The candidate chosen by `score` over the worlds `take` lets in: the
/// level's own (0) unless another's mean lead over the shared worlds beats
/// `margin`, then the largest lead.
fn choose(
    root: &Root,
    score: impl Fn(&Leaf) -> f64,
    take: impl Fn(usize) -> bool,
    margin: f64,
) -> usize {
    let Some(own) = root.candidates.get(&0) else {
        return 0;
    };
    let mut best = (0, margin);
    for (&at, tried) in root.candidates.iter().skip(1) {
        let leads: Vec<f64> = own
            .iter()
            .enumerate()
            .filter(|(world, _)| take(*world))
            .filter_map(|(_, (seed, a))| tried.get(seed).map(|b| score(b) - score(a)))
            .collect();
        if leads.is_empty() {
            continue;
        }
        #[allow(clippy::cast_precision_loss)]
        let lead = leads.iter().sum::<f64>() / leads.len() as f64;
        if lead > best.1 {
            best = (at, lead);
        }
    }
    best.0
}

/// The mean played-out lead of `chosen` over the level's own, over the
/// worlds `take` lets in; `None` where they share none.
fn lead(root: &Root, chosen: usize, take: impl Fn(usize) -> bool) -> Option<f64> {
    if chosen == 0 {
        return Some(0.0);
    }
    let own = root.candidates.get(&0)?;
    let tried = root.candidates.get(&chosen)?;
    let leads: Vec<f64> = own
        .iter()
        .enumerate()
        .filter(|(world, _)| take(*world))
        .filter_map(|(_, (seed, a))| tried.get(seed).map(|b| b.y - a.y))
        .collect();
    #[allow(clippy::cast_precision_loss)]
    let mean = (!leads.is_empty()).then(|| leads.iter().sum::<f64>() / leads.len() as f64);
    mean
}

#[derive(Default, Clone)]
struct Tally {
    roots: u64,
    replaced: u64,
    sum: f64,
    squares: f64,
}

impl Tally {
    fn add(&mut self, chosen: usize, lead: f64) {
        self.roots += 1;
        self.replaced += u64::from(chosen != 0);
        self.sum += lead;
        self.squares += lead * lead;
    }

    fn line(&self) -> String {
        #[allow(clippy::cast_precision_loss)]
        let n = self.roots.max(1) as f64;
        let mean = self.sum / n;
        let deviation = (self.squares / n - mean * mean).max(0.0).sqrt() / n.sqrt();
        format!(
            "{:>7} roots, {:>5.1} % replaced, lead per 100 roots {:>6.2} +- {:.2}",
            self.roots,
            100.0 * self.replaced as f64 / n,
            100.0 * mean,
            100.0 * deviation
        )
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let mut weights: Vec<Vec<f64>> = Vec::new();
    for path in &args.weights {
        let numbers: Vec<f64> = std::fs::read_to_string(path)
            .with_context(|| path.display().to_string())?
            .split_whitespace()
            .map(str::parse)
            .collect::<Result<_, _>>()?;
        anyhow::ensure!(numbers.len() == 2 * FEATURES, "{}", path.display());
        weights.push(numbers);
    }
    let models: Vec<Model> = args
        .trees
        .iter()
        .map(|path| Model::read(path))
        .collect::<Result<_, _>>()?;
    let mut roots: BTreeMap<u64, Root> = BTreeMap::new();
    for path in &args.log {
        let file = std::fs::File::open(path).with_context(|| path.display().to_string())?;
        for line in BufReader::new(file).lines() {
            let row: Row = serde_json::from_str(&line?)?;
            let late = late_share(row.turn);
            let mut values = vec![row.v];
            for weights in &weights {
                values.push(if row.t {
                    row.v
                } else {
                    let z: f64 = (0..FEATURES)
                        .map(|at| row.f[at] * (weights[at] + late * weights[FEATURES + at]))
                        .sum();
                    1.0 / (1.0 + (-z).exp())
                });
            }
            for model in &models {
                values.push(if row.t { row.v } else { model.value(&row.f, late) });
            }
            roots
                .entry(row.r)
                .or_insert_with(|| Root {
                    kind: row.k,
                    candidates: BTreeMap::new(),
                })
                .candidates
                .entry(row.c)
                .or_default()
                .insert(row.w, Leaf { t: row.t, y: row.y, values });
        }
    }
    let names: Vec<String> = std::iter::once("logged value".to_owned())
        .chain(args.weights.iter().map(|path| {
            path.file_stem()
                .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned())
        }))
        .chain(args.trees.iter().map(|path| {
            path.file_stem()
                .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned())
        }))
        .collect();
    let kinds = ["all", "priority", "attack", "block"];
    for (kind, kind_name) in kinds.iter().enumerate() {
        let of_kind = |root: &&Root| kind == 0 || root.kind + 1 == kind;
        println!("{kind_name}:");
        for (column, name) in names.iter().enumerate() {
            // Chosen on every world, judged on every world.
            let mut whole = Tally::default();
            // The same against the logged value's choice, root by root.
            let mut over = Tally::default();
            // Chosen on half the worlds, judged on the other half.
            let mut half = Tally::default();
            for root in roots.values().filter(of_kind) {
                let score = |leaf: &Leaf| leaf.values[column];
                let chosen = choose(root, score, |_| true, args.margin);
                if let Some(lead) = lead(root, chosen, |_| true) {
                    whole.add(chosen, lead);
                    let logged = choose(root, |leaf: &Leaf| leaf.values[0], |_| true, args.margin);
                    if let Some(base) = self::lead(root, logged, |_| true) {
                        over.add(usize::from(chosen != logged), lead - base);
                    }
                }
                for parity in 0..2 {
                    let chosen = choose(root, score, |world| world % 2 == parity, args.margin);
                    if let Some(lead) = lead(root, chosen, |world| world % 2 != parity) {
                        half.add(chosen, lead);
                    }
                }
            }
            println!("  {name:<28} {}", whole.line());
            println!("  {:<28} {}", "  over the logged value", over.line());
            println!("  {:<28} {}", "  on half, judged on half", half.line());
        }
        let mut played = Tally::default();
        for root in roots.values().filter(of_kind) {
            for parity in 0..2 {
                let chosen = choose(root, |leaf| leaf.y, |world| world % 2 == parity, 0.0);
                if let Some(lead) = lead(root, chosen, |world| world % 2 != parity) {
                    played.add(chosen, lead);
                }
            }
        }
        println!("  {:<28} {}", "played out, half on half", played.line());
    }
    // Where the logged value's lead comes from: its replaced roots by how
    // far the leaf put the chosen answer ahead, and by whether a world of
    // either answer ended before its leaf.
    println!("the logged value's replaced roots, by the leaf's lead and by an ended world:");
    let mut buckets: BTreeMap<(usize, bool), Tally> = BTreeMap::new();
    let mut all = 0_u64;
    for root in roots.values() {
        all += 1;
        let chosen = choose(root, |leaf: &Leaf| leaf.values[0], |_| true, args.margin);
        if chosen == 0 {
            continue;
        }
        let Some(played) = lead(root, chosen, |_| true) else {
            continue;
        };
        let own = &root.candidates[&0];
        let tried = &root.candidates[&chosen];
        let leads: Vec<f64> = own
            .iter()
            .filter_map(|(seed, a)| tried.get(seed).map(|b| b.values[0] - a.values[0]))
            .collect();
        #[allow(clippy::cast_precision_loss)]
        let by_leaf = leads.iter().sum::<f64>() / leads.len().max(1) as f64;
        let bucket = [0.04, 0.08, 0.16, 0.32]
            .iter()
            .position(|edge| by_leaf < *edge)
            .unwrap_or(4);
        let ended = own.values().chain(tried.values()).any(|leaf| leaf.t);
        buckets.entry((bucket, ended)).or_default().add(chosen, played);
    }
    for ((bucket, ended), tally) in &buckets {
        #[allow(clippy::cast_precision_loss)]
        let share = 100.0 * tally.sum / all.max(1) as f64;
        println!(
            "  leaf lead {} {}: {} roots, played-out lead {:.3} a root, {share:.3} per 100 roots of all",
            ["0.02-0.04", "0.04-0.08", "0.08-0.16", "0.16-0.32", "0.32+"][*bucket],
            if *ended { "with an ended world" } else { "all worlds at a leaf" },
            tally.roots,
            tally.sum / tally.roots.max(1) as f64
        );
    }
    Ok(())
}

/// A linear value with boosted depth-2 trees on top, as `fit_leaf_trees`
/// writes it.
struct Model {
    rate: f64,
    linear: Vec<f64>,
    /// Root column and cut, the two children's (column -1: none), and the
    /// four leaves.
    trees: Vec<[f64; 10]>,
}

impl Model {
    fn read(path: &std::path::Path) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path).with_context(|| path.display().to_string())?;
        let mut lines = text.lines();
        let rate = lines.next().context("the rate")?.trim().parse()?;
        let linear: Vec<f64> = lines
            .next()
            .context("the linear weights")?
            .split_whitespace()
            .map(str::parse)
            .collect::<Result<_, _>>()?;
        let mut trees = Vec::new();
        for line in lines {
            let numbers: Vec<f64> = line
                .split_whitespace()
                .map(str::parse)
                .collect::<Result<_, _>>()?;
            trees.push(<[f64; 10]>::try_from(numbers).ok().context("a tree")?);
        }
        Ok(Self {
            rate,
            linear,
            trees,
        })
    }

    fn value(&self, f: &[f64], late: f64) -> f64 {
        let read = |column: f64, sign: f64| {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let column = column as usize;
            if column == FEATURES {
                late
            } else {
                sign * f[column]
            }
        };
        let leaf = |tree: &[f64; 10], sign: f64| {
            let side = usize::from(read(tree[0], sign) > tree[1]);
            let (column, at) = (tree[2 + 2 * side], tree[3 + 2 * side]);
            let below = usize::from(column >= 0.0 && read(column, sign) > at);
            tree[6 + 2 * side + below]
        };
        let grown: f64 = self
            .trees
            .iter()
            .map(|tree| leaf(tree, 1.0) - leaf(tree, -1.0))
            .sum();
        let z: f64 = (0..FEATURES)
            .map(|at| f[at] * (self.linear[at] + late * self.linear[FEATURES + at]))
            .sum::<f64>()
            + 0.5 * self.rate * grown;
        1.0 / (1.0 + (-z).exp())
    }
}
