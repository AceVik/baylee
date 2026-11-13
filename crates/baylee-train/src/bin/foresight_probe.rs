//! What the house search's choices are worth by the games played on from
//! them, read from a probe log (`eval4 --house-foresight --foresight-probe`).
//!
//! A probed question has every world of every answer tried played on past
//! its leaf to the end of the game. So for each answer there is what the
//! leaf value made of each world and how each world came out, and a choice
//! can be judged by results instead of by the value that made it:
//!
//! - the **lead** of a choice is how much more often the chosen answer's
//!   worlds were won than the level's own answer's, over the worlds both
//!   have; a question where the level's own answer stands has a lead of 0.
//!   Summed over questions it is what the search gains, in games won per
//!   question asked, as far as the worlds and the house's play in them say;
//! - *on half, judged on half* chooses on every second world and judges on
//!   the others, both ways round: a choice does not get to be judged in the
//!   deals it was fitted to, which is how the real game treats it;
//! - *played out* chooses by the results themselves on half the worlds: what
//!   a value that knew each world's end would get from so few worlds;
//! - `--weights` (as `fit_leaf --out` writes them) values the leaves again
//!   with other weights and reports that value's choices beside the logged
//!   one's, and question by question against it.
//!
//! Then the logged value's replaced answers by how far ahead the leaf put
//! them and by whether a world of either answer ended before its leaf: where
//! the lead comes from.
//!
//! The games are the house's own forks: this measures the search, with a
//! house player in every chair, and says nothing about any opponent.

#![allow(clippy::missing_docs_in_private_items)]

use std::collections::BTreeMap;
use std::io::{BufRead as _, BufReader};
use std::path::PathBuf;

use anyhow::Context as _;
use baylee_ai::foresight::FEATURES;
use clap::Parser;

#[derive(Parser)]
#[command(about = "Judges the house search's choices by the games played on from them")]
struct Args {
    /// The probe logs.
    #[arg(long, required = true, num_args = 1..)]
    log: Vec<PathBuf>,
    /// Weights to value the leaves with again, a file each.
    #[arg(long, num_args = 0..)]
    weights: Vec<PathBuf>,
    /// The lead in value another answer has to show over the level's own.
    #[arg(long, default_value_t = 0.02)]
    margin: f64,
}

#[derive(serde::Deserialize)]
struct Row {
    #[serde(default)]
    g: u64,
    r: u64,
    w: u64,
    c: usize,
    k: usize,
    t: bool,
    y: f64,
    v: f64,
    f: Vec<f64>,
}

/// One world of one answer: whether it ended before its leaf, how it came
/// out, and the leaf's value as logged and under each set of weights.
struct Leaf {
    ended: bool,
    played: f64,
    values: Vec<f64>,
}

/// A question's answers, each by world.
struct Root {
    kind: usize,
    answers: BTreeMap<usize, BTreeMap<u64, Leaf>>,
}

impl Root {
    /// The mean of `score` of answer `chosen` less that of the level's own,
    /// over the worlds both have that `take` lets in (by their place among
    /// the own answer's worlds); `None` where there are none.
    fn lead(
        &self,
        chosen: usize,
        score: impl Fn(&Leaf) -> f64,
        take: impl Fn(usize) -> bool,
    ) -> Option<f64> {
        let own = self.answers.get(&0)?;
        let tried = self.answers.get(&chosen)?;
        let leads: Vec<f64> = own
            .iter()
            .enumerate()
            .filter(|(place, _)| take(*place))
            .filter_map(|(_, (world, a))| tried.get(world).map(|b| score(b) - score(a)))
            .collect();
        #[allow(clippy::cast_precision_loss)]
        let mean = (!leads.is_empty()).then(|| leads.iter().sum::<f64>() / leads.len() as f64);
        mean
    }

    /// The answer the search gives by `score`: the level's own (0) unless
    /// another's lead beats `margin`, then the largest lead, the earlier on
    /// a tie (`baylee_ai::foresight::choose`).
    fn choose(
        &self,
        score: impl Fn(&Leaf) -> f64,
        take: impl Fn(usize) -> bool,
        margin: f64,
    ) -> usize {
        let mut best = (0, margin);
        for &at in self.answers.keys().filter(|at| **at != 0) {
            if let Some(lead) = self.lead(at, &score, &take)
                && lead > best.1
            {
                best = (at, lead);
            }
        }
        best.0
    }

    /// Whether a world of answer `at` came out other than the same world of
    /// the level's own answer.
    fn differs(&self, at: usize) -> bool {
        let (Some(own), Some(tried)) = (self.answers.get(&0), self.answers.get(&at)) else {
            return false;
        };
        own.iter().any(|(world, a)| {
            tried
                .get(world)
                .is_some_and(|b| (a.played - b.played).abs() > f64::EPSILON)
        })
    }

    /// The played-out lead of `chosen`: 0 for the level's own answer.
    fn played(&self, chosen: usize, take: impl Fn(usize) -> bool) -> Option<f64> {
        if chosen == 0 {
            return self.answers.contains_key(&0).then_some(0.0);
        }
        self.lead(chosen, |leaf| leaf.played, take)
    }
}

/// Questions, how many had their answer replaced, and the leads' sum and
/// sum of squares.
#[derive(Default)]
struct Tally {
    roots: u64,
    replaced: u64,
    sum: f64,
    squares: f64,
}

impl Tally {
    fn add(&mut self, replaced: bool, lead: f64) {
        self.roots += 1;
        self.replaced += u64::from(replaced);
        self.sum += lead;
        self.squares += lead * lead;
    }

    /// The mean lead per 100 questions with its standard error.
    fn line(&self) -> String {
        #[allow(clippy::cast_precision_loss)]
        let n = self.roots.max(1) as f64;
        let mean = self.sum / n;
        let error = (self.squares / n - mean * mean).max(0.0).sqrt() / n.sqrt();
        #[allow(clippy::cast_precision_loss)]
        let replaced = 100.0 * self.replaced as f64 / n;
        format!(
            "{:>7} questions, {replaced:>5.1} % replaced, lead per 100 questions {:>6.2} +- {:.2}",
            self.roots,
            100.0 * mean,
            100.0 * error
        )
    }
}

fn read(args: &Args) -> anyhow::Result<BTreeMap<(u64, u64), Root>> {
    let mut weights: Vec<Vec<f64>> = Vec::new();
    for path in &args.weights {
        let numbers: Vec<f64> = std::fs::read_to_string(path)
            .with_context(|| path.display().to_string())?
            .split_whitespace()
            .map(str::parse)
            .collect::<Result<_, _>>()?;
        anyhow::ensure!(
            numbers.len() == FEATURES,
            "{}: {} weights, this build's leaf has {FEATURES}",
            path.display(),
            numbers.len()
        );
        weights.push(numbers);
    }
    let mut roots: BTreeMap<(u64, u64), Root> = BTreeMap::new();
    let mut offset = 0;
    for path in &args.log {
        let file = std::fs::File::open(path).with_context(|| path.display().to_string())?;
        let mut tables = 0;
        for line in BufReader::new(file).lines() {
            let row: Row = serde_json::from_str(&line?)?;
            anyhow::ensure!(row.f.len() == FEATURES, "{}: features", path.display());
            tables = tables.max(row.g + 1);
            let mut values = vec![row.v];
            for weights in &weights {
                values.push(if row.t {
                    row.v
                } else {
                    let z: f64 = row.f.iter().zip(weights).map(|(x, w)| x * w).sum();
                    1.0 / (1.0 + (-z).exp())
                });
            }
            roots
                .entry((offset + row.g, row.r))
                .or_insert_with(|| Root {
                    kind: row.k,
                    answers: BTreeMap::new(),
                })
                .answers
                .entry(row.c)
                .or_default()
                .insert(
                    row.w,
                    Leaf {
                        ended: row.t,
                        played: row.y,
                        values,
                    },
                );
        }
        offset += tables;
    }
    Ok(roots)
}

/// Where the logged value's lead comes from: its replaced answers by how
/// far ahead the leaf put them and by whether a world ended before its leaf.
fn sources(roots: &BTreeMap<(u64, u64), Root>, margin: f64) {
    const EDGES: [f64; 4] = [0.04, 0.08, 0.16, 0.32];
    const NAMES: [&str; 5] = [
        "under 0.04",
        "0.04-0.08",
        "0.08-0.16",
        "0.16-0.32",
        "0.32 up",
    ];
    let mut buckets: BTreeMap<(usize, bool), Tally> = BTreeMap::new();
    for root in roots.values() {
        let chosen = root.choose(|leaf| leaf.values[0], |_| true, margin);
        if chosen == 0 {
            continue;
        }
        let (Some(played), Some(valued)) = (
            root.played(chosen, |_| true),
            root.lead(chosen, |leaf| leaf.values[0], |_| true),
        ) else {
            continue;
        };
        let bucket = EDGES
            .iter()
            .position(|edge| valued < *edge)
            .unwrap_or(EDGES.len());
        let ended = [0, chosen]
            .iter()
            .filter_map(|at| root.answers.get(at))
            .flat_map(BTreeMap::values)
            .any(|leaf| leaf.ended);
        buckets
            .entry((bucket, ended))
            .or_default()
            .add(true, played);
    }
    // The questions where it did not matter what was answered: every world
    // of every answer tried came out as the level's own answer's did.
    let same = roots
        .values()
        .filter(|root| root.answers.keys().all(|&at| !root.differs(at)))
        .count();
    println!(
        "{same} of {} questions came out the same in every world whatever was answered",
        roots.len()
    );
    println!("the logged value's replaced answers, by the leaf's lead:");
    #[allow(clippy::cast_precision_loss)]
    let all = roots.len().max(1) as f64;
    for ((bucket, ended), tally) in &buckets {
        #[allow(clippy::cast_precision_loss)]
        let each = tally.sum / tally.roots.max(1) as f64;
        println!(
            "  {:<10} {:<22} {:>5} answers, played-out lead {each:>6.3} each, {:>6.3} per 100 questions",
            NAMES[*bucket],
            if *ended {
                "a world ended early"
            } else {
                "every world at a leaf"
            },
            tally.roots,
            100.0 * tally.sum / all
        );
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let roots = read(&args)?;
    let names: Vec<String> = std::iter::once("logged value".to_owned())
        .chain(args.weights.iter().map(|path| {
            path.file_stem()
                .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned())
        }))
        .collect();
    for (kind, kind_name) in ["all", "priority", "attack", "block"].iter().enumerate() {
        let of_kind = |root: &&Root| kind == 0 || root.kind + 1 == kind;
        println!("{kind_name}:");
        for (column, name) in names.iter().enumerate() {
            let score = |leaf: &Leaf| leaf.values[column];
            let (mut whole, mut over, mut half) =
                (Tally::default(), Tally::default(), Tally::default());
            for root in roots.values().filter(of_kind) {
                let chosen = root.choose(score, |_| true, args.margin);
                if let Some(lead) = root.played(chosen, |_| true) {
                    whole.add(chosen != 0, lead);
                    let logged = root.choose(|leaf| leaf.values[0], |_| true, args.margin);
                    if let Some(base) = root.played(logged, |_| true) {
                        over.add(chosen != logged, lead - base);
                    }
                }
                for parity in 0..2 {
                    let chosen = root.choose(score, |place| place % 2 == parity, args.margin);
                    if let Some(lead) = root.played(chosen, |place| place % 2 != parity) {
                        half.add(chosen != 0, lead);
                    }
                }
            }
            println!("  {name:<26} {}", whole.line());
            if column > 0 {
                println!("  {:<26} {}", "  against the logged", over.line());
            }
            println!("  {:<26} {}", "  on half, judged on half", half.line());
        }
        let mut played = Tally::default();
        for root in roots.values().filter(of_kind) {
            for parity in 0..2 {
                let chosen = root.choose(|leaf| leaf.played, |place| place % 2 == parity, 0.0);
                if let Some(lead) = root.played(chosen, |place| place % 2 != parity) {
                    played.add(chosen != 0, lead);
                }
            }
        }
        println!("  {:<26} {}", "played out, half on half", played.line());
    }
    sources(&roots, args.margin);
    Ok(())
}
