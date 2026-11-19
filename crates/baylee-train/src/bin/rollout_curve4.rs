//! How much of the net's own search a cheaper rollout net keeps, read from
//! the search events of an `eval4 --rollout-compare` run (`games.jsonl`):
//! at every searched root the net's search (`worlds`, the net in every chair)
//! and, on the same worlds, the rollout net's search under each leaf
//! (`variants.<leaf>.worlds`).
//!
//! Judged as the live-search curve judges a cheaper search
//! (`search_labels::halves`): a search chooses on the even worlds (the best
//! mean advantage over the policy's answer, if it clears the margin) and is
//! judged by the NET's scores on the odd worlds, and the other way round; a
//! root's lead is the mean of the two. The net's full lead is its own
//! choice so judged. Per 100 roots, with the standard error by game; a
//! leaf's kept share is its lead over the full lead on the same roots.
//!
//! The games split by their deal: even deals are the selection half, odd
//! deals the report half. The leaf chosen is the cheapest (`student`, then
//! `jump`, then `follow`) whose selection-half share is within `--tie` of
//! the best; it passes when its report-half share is at least `--keep` and
//! its lead is at least `--z` standard errors. Priority roots are judged;
//! combat declarations are reported apart.

#![allow(clippy::missing_docs_in_private_items)]

use std::collections::BTreeMap;
use std::io::{BufRead as _, BufReader};
use std::path::PathBuf;

use anyhow::Context as _;
use baylee_train::search_labels::{HALVES_MARGIN, advantage, choose, halves};
use clap::Parser;
use serde_json::{Value, json};

#[derive(Parser)]
#[command(about = "Judges a rollout net's searches by the net's own worlds")]
struct Args {
    /// `games.jsonl` files of `eval4 --rollout-compare` runs.
    #[arg(long, required = true, num_args = 1..)]
    games: Vec<PathBuf>,
    /// The mean advantage an answer must show to replace the policy's.
    #[arg(long, default_value_t = HALVES_MARGIN)]
    margin: f64,
    /// The report-half share of the full lead the chosen leaf must keep.
    #[arg(long, default_value_t = 0.70)]
    keep: f64,
    /// The standard errors its report-half lead must clear.
    #[arg(long, default_value_t = 2.0)]
    z: f64,
    /// A cheaper leaf within this share of the best is chosen instead.
    #[arg(long, default_value_t = 0.05)]
    tie: f64,
    /// Where the report goes, as JSON; it is printed either way.
    #[arg(long)]
    out: Option<PathBuf>,
}

/// The leaves, cheapest first.
const LEAVES: [&str; 3] = ["student", "jump", "follow"];

#[derive(serde::Deserialize)]
struct Game {
    i: u64,
    #[serde(default)]
    search_events: Vec<Value>,
}

/// One root's leads: the net's own, and each leaf's.
struct Root {
    game: u64,
    combat: bool,
    full: f64,
    leaves: BTreeMap<&'static str, f64>,
    agrees: BTreeMap<&'static str, bool>,
    steps: BTreeMap<&'static str, u64>,
}

fn worlds(value: &Value) -> Option<Vec<Vec<f64>>> {
    serde_json::from_value(value.clone()).ok()
}

/// The root of a search event, when the net's and every leaf's search
/// played the same candidates in four complete worlds; why not otherwise.
fn root(game: u64, event: &Value, margin: f64) -> Result<Root, &'static str> {
    let own = worlds(&event["worlds"]).ok_or("no worlds")?;
    if own.len() != 4 {
        return Err("incomplete_worlds");
    }
    let candidates = &event["candidates"];
    let combat = candidates[0]
        .as_object()
        .is_some_and(|c| c.contains_key("DeclareAttackers") || c.contains_key("DeclareBlockers"));
    let even: Vec<&[f64]> = own.iter().step_by(2).map(Vec::as_slice).collect();
    let odd: Vec<&[f64]> = own.iter().skip(1).step_by(2).map(Vec::as_slice).collect();
    let full = halves(&own, margin);
    let mut root = Root {
        game,
        combat,
        full: full.lead(),
        leaves: BTreeMap::new(),
        agrees: BTreeMap::new(),
        steps: BTreeMap::new(),
    };
    for leaf in LEAVES {
        let variant = &event["variants"][leaf];
        if variant.is_null() {
            continue;
        }
        if &variant["candidates"] != candidates {
            return Err("other_candidates");
        }
        let theirs = worlds(&variant["worlds"]).ok_or("variant_unsearched")?;
        if theirs.len() != 4 {
            return Err("incomplete_variant_worlds");
        }
        let their_even: Vec<&[f64]> = theirs.iter().step_by(2).map(Vec::as_slice).collect();
        let their_odd: Vec<&[f64]> = theirs
            .iter()
            .skip(1)
            .step_by(2)
            .map(Vec::as_slice)
            .collect();
        let on_even = choose(&their_even, margin);
        let on_odd = choose(&their_odd, margin);
        let lead = f64::midpoint(advantage(&odd, on_even), advantage(&even, on_odd));
        root.leaves.insert(leaf, lead);
        root.agrees
            .insert(leaf, on_even == full.on_even && on_odd == full.on_odd);
        root.steps
            .insert(leaf, variant["rollout_steps"].as_u64().unwrap_or(0));
    }
    Ok(root)
}

/// Per 100 roots: the lead, its standard error by game, and the roots.
#[allow(clippy::cast_precision_loss, reason = "counts far below 2^53")]
fn per_100(roots: &[&Root], lead: impl Fn(&Root) -> f64) -> (f64, f64, usize) {
    let mut by_game: BTreeMap<u64, f64> = BTreeMap::new();
    for root in roots {
        *by_game.entry(root.game).or_default() += lead(root);
    }
    let r = roots.len() as f64;
    if roots.is_empty() {
        return (0.0, 0.0, 0);
    }
    let total: f64 = by_game.values().sum();
    let g = by_game.len() as f64;
    let mean = total / g;
    let var = if by_game.len() > 1 {
        by_game.values().map(|x| (x - mean).powi(2)).sum::<f64>() / (g - 1.0)
    } else {
        0.0
    };
    (100.0 * total / r, (g * var).sqrt() / r * 100.0, roots.len())
}

/// One half's table: the full lead and each leaf's lead, share and z.
#[allow(clippy::cast_precision_loss, reason = "counts far below 2^53")]
fn half(roots: &[&Root]) -> Value {
    let (full, full_se, n) = per_100(roots, |root| root.full);
    let mut leaves = serde_json::Map::new();
    for leaf in LEAVES {
        let present: Vec<&Root> = roots
            .iter()
            .copied()
            .filter(|root| root.leaves.contains_key(leaf))
            .collect();
        if present.is_empty() {
            continue;
        }
        let (lead, se, _) = per_100(&present, |root| root.leaves[leaf]);
        let (full_here, _, _) = per_100(&present, |root| root.full);
        let agree =
            present.iter().filter(|root| root.agrees[leaf]).count() as f64 / present.len() as f64;
        let steps =
            present.iter().map(|root| root.steps[leaf]).sum::<u64>() as f64 / present.len() as f64;
        leaves.insert(
            leaf.to_owned(),
            json!({
                "roots": present.len(),
                "lead_per_100": lead,
                "se": se,
                "z": if se > 0.0 { lead / se } else { 0.0 },
                "full_per_100": full_here,
                "kept": (full_here > 0.0).then(|| lead / full_here),
                "same_choices_as_the_net": agree,
                "rollout_steps_per_root": steps,
            }),
        );
    }
    json!({"roots": n, "full_per_100": full, "full_se": full_se, "leaves": leaves})
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let mut roots = Vec::new();
    let mut dropped: BTreeMap<&'static str, u64> = BTreeMap::new();
    for path in &args.games {
        let file = std::fs::File::open(path).with_context(|| format!("{}", path.display()))?;
        for line in BufReader::new(file).lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let game: Game = serde_json::from_str(&line)
                .with_context(|| format!("a game of {}", path.display()))?;
            for event in &game.search_events {
                match root(game.i, event, args.margin) {
                    Ok(root) => roots.push(root),
                    Err(why) => *dropped.entry(why).or_default() += 1,
                }
            }
        }
    }
    let priority = |selection: bool| -> Vec<&Root> {
        roots
            .iter()
            .filter(|root| !root.combat && (root.game % 2 == 0) == selection)
            .collect()
    };
    let (selection, report) = (half(&priority(true)), half(&priority(false)));
    let kept = |table: &Value, leaf: &str| table["leaves"][leaf]["kept"].as_f64();
    let best = LEAVES
        .iter()
        .filter_map(|leaf| kept(&selection, leaf))
        .fold(f64::NEG_INFINITY, f64::max);
    let chosen = LEAVES
        .iter()
        .find(|leaf| kept(&selection, leaf).is_some_and(|k| k >= best - args.tie))
        .copied();
    let verdict = chosen.map(|leaf| {
        let on_report = &report["leaves"][leaf];
        let kept = on_report["kept"].as_f64().unwrap_or(0.0);
        let z = on_report["z"].as_f64().unwrap_or(0.0);
        json!({"leaf": leaf, "kept": kept, "z": z,
            "pass": kept >= args.keep && z >= args.z})
    });
    let combat: Vec<&Root> = roots.iter().filter(|root| root.combat).collect();
    let report_json = json!({
        "contract": "rollout_net_curve_v1",
        "margin": args.margin,
        "rule": {"keep": args.keep, "z": args.z, "tie": args.tie,
            "halves": "even deals choose the leaf, odd deals report",
            "population": "priority roots; combat declarations informational"},
        "roots": roots.len(),
        "dropped": dropped,
        "selection": selection,
        "report": report,
        "chosen": verdict,
        "informational_combat": half(&combat),
        "informational_all_priority": half(&roots.iter().filter(|r| !r.combat).collect::<Vec<_>>()),
    });
    let text = serde_json::to_string_pretty(&report_json)?;
    println!("{text}");
    if let Some(out) = &args.out {
        std::fs::write(out, text + "\n")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(own: &[[f64; 3]; 4], leaf: &[[f64; 3]; 4]) -> Value {
        let candidates =
            json!(["PassPriority", {"PlayLand": {"card": 1}}, {"PlayLand": {"card": 2}}]);
        json!({"candidates": candidates, "worlds": own,
            "variants": {"jump": {"candidates": candidates, "worlds": leaf, "rollout_steps": 40}}})
    }

    #[test]
    fn a_leaf_is_judged_by_the_nets_worlds_not_its_own() {
        // The net: candidate 2 leads by 0.1 in every world.
        let own = [[0.5, 0.5, 0.6]; 4];
        // The leaf's own worlds prefer candidate 1, which the net scores even.
        let leaf = [[0.5, 0.7, 0.5]; 4];
        let root = root(0, &event(&own, &leaf), 0.03).unwrap();
        assert!((root.full - 0.1).abs() < 1e-12);
        assert!(
            root.leaves["jump"].abs() < 1e-12,
            "judged by the net, it gains nothing"
        );
        assert!(!root.agrees["jump"]);
        let same = root_of(&own);
        assert!((same.leaves["jump"] - 0.1).abs() < 1e-12);
        assert!(same.agrees["jump"]);
    }

    fn root_of(own: &[[f64; 3]; 4]) -> Root {
        root(1, &event(own, own), 0.03).unwrap()
    }

    #[test]
    fn other_candidates_or_missing_worlds_drop_the_root() {
        let own = [[0.5, 0.5, 0.6]; 4];
        let mut moved = event(&own, &own);
        moved["variants"]["jump"]["candidates"] = json!(["PassPriority"]);
        assert_eq!(root(0, &moved, 0.03).err(), Some("other_candidates"));
        let mut short = event(&own, &own);
        short["worlds"] = json!([[0.5, 0.5, 0.6]]);
        assert_eq!(root(0, &short, 0.03).err(), Some("incomplete_worlds"));
    }

    #[test]
    fn per_100_takes_its_error_by_game() {
        let own = [[0.5, 0.5, 0.6]; 4];
        let roots = [
            root(0, &event(&own, &own), 0.03).unwrap(),
            root(2, &event(&own, &own), 0.03).unwrap(),
        ];
        let refs: Vec<&Root> = roots.iter().collect();
        let (lead, se, n) = per_100(&refs, |root| root.full);
        assert_eq!(n, 2);
        assert!((lead - 10.0).abs() < 1e-9);
        assert!(se.abs() < 1e-9, "two games with the same lead");
    }
}
