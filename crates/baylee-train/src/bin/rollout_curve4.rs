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
//! choice so judged. Per 100 roots, with `fork_advantage4`'s standard error
//! by game (the ratio estimator's, one cluster a games file and deal); a
//! leaf's kept share is its lead over the full lead on the same roots.
//!
//! The games split by their table, which is its seed: tables of an even seed
//! are the selection half, tables of an odd seed the report half. The deals
//! of a table seat the net in another chair each and play the same decks from
//! the same shuffle, so they are in one half together. The leaf chosen is the
//! cheapest (`student`, then `jump`, then `follow`) whose selection-half
//! share is within `--tie` of the best; it passes when its report-half share
//! is at least `--keep` and its lead is at least `--z` standard errors.
//! Priority roots are judged; combat declarations are reported apart.
//!
//! An event whose answer the table refused is left out, and so is one whose
//! worlds are not complete. A leaf that did not search a root, or searched
//! other candidates, is left out there alone. What was left out is counted,
//! and a run that leaves no judged leaf at all is an error.

#![allow(clippy::missing_docs_in_private_items)]

#[path = "fork_advantage4/leads.rs"]
mod leads;

use std::collections::BTreeMap;
use std::io::{BufRead as _, BufReader};
use std::path::{Path, PathBuf};

use anyhow::Context as _;
use baylee_train::search_labels::{HALVES_MARGIN, advantage, choose, halves};
use clap::Parser;
use serde_json::{Value, json};

use leads::{Leads, real};

#[derive(Parser)]
#[command(about = "Judges a rollout net's searches by the net's own worlds")]
struct Args {
    /// `games.jsonl` files of `eval4 --rollout-compare` runs.
    #[arg(long, required = true, num_args = 1..)]
    games: Vec<PathBuf>,
    /// The worlds of a complete root: the runs' P (`--search K,P,H`).
    #[arg(long, default_value_t = 4, value_parser = clap::value_parser!(u32).range(2..))]
    worlds: u32,
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
    /// The deal.
    i: u64,
    /// The table's seed, the same for every deal of it.
    seed: u64,
    #[serde(default)]
    search_events: Vec<Value>,
}

/// A search's choices on each half of a root's worlds, judged by the net's
/// scores on the other.
struct Judged {
    lead: f64,
    /// Whether a half replaced the policy's answer.
    replaced: bool,
}

/// A leaf's search of a root.
struct Leaf {
    judged: Judged,
    /// Whether both halves chose what the net's own halves chose.
    agrees: bool,
    steps: u64,
}

/// One root: the net's own search and each leaf's.
struct Root {
    combat: bool,
    full: Judged,
    leaves: BTreeMap<&'static str, Leaf>,
    /// The leaves that do not judge this root, and why.
    left_out: BTreeMap<&'static str, &'static str>,
}

/// A root and the game it is of.
struct Placed {
    /// Its cluster: a games file and deal.
    game: u64,
    /// Whether its table is of the selection half.
    selection: bool,
    root: Root,
}

/// A root's scores, when every one of `worlds` worlds scores each of `width`
/// candidates; why not otherwise.
fn complete(value: &Value, worlds: usize, width: usize) -> Result<Vec<Vec<f64>>, &'static str> {
    let scores: Vec<Vec<f64>> = serde_json::from_value(value.clone()).map_err(|_| "no_worlds")?;
    if scores.len() != worlds || scores.iter().any(|world| world.len() != width) {
        return Err("incomplete_worlds");
    }
    Ok(scores)
}

/// The even worlds, or the odd ones.
fn half(worlds: &[Vec<f64>], parity: usize) -> Vec<&[f64]> {
    worlds
        .iter()
        .skip(parity)
        .step_by(2)
        .map(Vec::as_slice)
        .collect()
}

/// A leaf's search of a root whose net scored `own` and chose `net` on the
/// even and the odd worlds; why it does not judge the root otherwise.
fn leaf(
    variant: &Value,
    candidates: &Value,
    own: &[Vec<f64>],
    net: (usize, usize),
    margin: f64,
) -> Result<Leaf, &'static str> {
    if !variant["unsearched"].is_null() {
        return Err("unsearched");
    }
    if &variant["candidates"] != candidates {
        return Err("other_candidates");
    }
    let theirs = complete(&variant["worlds"], own.len(), own[0].len())?;
    let chosen = (
        choose(&half(&theirs, 0), margin),
        choose(&half(&theirs, 1), margin),
    );
    Ok(Leaf {
        judged: Judged {
            lead: f64::midpoint(
                advantage(&half(own, 1), chosen.0),
                advantage(&half(own, 0), chosen.1),
            ),
            replaced: chosen != (0, 0),
        },
        agrees: chosen == net,
        steps: variant["rollout_steps"].as_u64().unwrap_or(0),
    })
}

/// The root of a search event, with every leaf that judges it; why the event
/// is left out otherwise.
fn root(event: &Value, worlds: usize, margin: f64) -> Result<Root, &'static str> {
    if event["accepted"] == false {
        return Err("refused_by_the_table");
    }
    let candidates = &event["candidates"];
    let width = candidates.as_array().map_or(0, Vec::len);
    if width == 0 {
        return Err("no_candidates");
    }
    let own = complete(&event["worlds"], worlds, width)?;
    let combat = candidates[0]
        .as_object()
        .is_some_and(|c| c.contains_key("DeclareAttackers") || c.contains_key("DeclareBlockers"));
    let full = halves(&own, margin);
    let mut root = Root {
        combat,
        full: Judged {
            lead: full.lead(),
            replaced: full.on_even != 0 || full.on_odd != 0,
        },
        leaves: BTreeMap::new(),
        left_out: BTreeMap::new(),
    };
    for name in LEAVES {
        let variant = &event["variants"][name];
        if variant.is_null() {
            // The run did not compare this leaf.
            continue;
        }
        match leaf(
            variant,
            candidates,
            &own,
            (full.on_even, full.on_odd),
            margin,
        ) {
            Ok(leaf) => {
                root.leaves.insert(name, leaf);
            }
            Err(why) => {
                root.left_out.insert(name, why);
            }
        }
    }
    Ok(root)
}

/// The roots of every games file, and what was left out.
#[derive(Default)]
struct Roots {
    placed: Vec<Placed>,
    /// Events left out, by why.
    events: BTreeMap<&'static str, u64>,
    /// Leaves left out of roots, by leaf and why.
    leaves: BTreeMap<&'static str, BTreeMap<&'static str, u64>>,
    /// The cluster of each games file and deal.
    clusters: BTreeMap<(usize, u64), u64>,
}

impl Roots {
    fn game(&mut self, file: usize, game: &Game, args: &Args) {
        let next = self.clusters.len() as u64;
        let cluster = *self.clusters.entry((file, game.i)).or_insert(next);
        for event in &game.search_events {
            match root(event, args.worlds as usize, args.margin) {
                Ok(root) => {
                    for (&leaf, &why) in &root.left_out {
                        *self.leaves.entry(leaf).or_default().entry(why).or_default() += 1;
                    }
                    self.placed.push(Placed {
                        game: cluster,
                        selection: game.seed.is_multiple_of(2),
                        root,
                    });
                }
                Err(why) => *self.events.entry(why).or_default() += 1,
            }
        }
    }

    fn file(&mut self, file: usize, path: &Path, args: &Args) -> anyhow::Result<()> {
        let open = std::fs::File::open(path).with_context(|| format!("{}", path.display()))?;
        for line in BufReader::new(open).lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let game: Game = serde_json::from_str(&line)
                .with_context(|| format!("a game of {}", path.display()))?;
            self.game(file, &game, args);
        }
        Ok(())
    }

    /// # Errors
    /// When no leaf judges any root: there is nothing to report on.
    fn ensure_judged(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.placed
                .iter()
                .any(|placed| !placed.root.leaves.is_empty()),
            "no root has a judged leaf: events left out {:?}, leaves left out {:?}",
            self.events,
            self.leaves
        );
        Ok(())
    }

    fn report(&self, args: &Args) -> Value {
        let priority = |selection: bool| -> Vec<&Placed> {
            self.placed
                .iter()
                .filter(|placed| !placed.root.combat && placed.selection == selection)
                .collect()
        };
        let (selection, report) = (table(&priority(true)), table(&priority(false)));
        let kept = |table: &Value, leaf: &str| table["leaves"][leaf]["kept"].as_f64();
        let best = LEAVES
            .iter()
            .filter_map(|leaf| kept(&selection, leaf))
            .fold(f64::NEG_INFINITY, f64::max);
        let chosen = LEAVES
            .iter()
            .find(|leaf| kept(&selection, leaf).is_some_and(|k| k >= best - args.tie));
        let verdict = chosen.map(|leaf| {
            let on_report = &report["leaves"][leaf];
            let kept = on_report["kept"].as_f64().unwrap_or(0.0);
            let z = on_report["z"].as_f64().unwrap_or(0.0);
            json!({"leaf": leaf, "kept": kept, "z": z,
                "pass": kept >= args.keep && z >= args.z})
        });
        let combat: Vec<&Placed> = self.placed.iter().filter(|p| p.root.combat).collect();
        let every: Vec<&Placed> = self.placed.iter().filter(|p| !p.root.combat).collect();
        json!({
            "contract": "rollout_net_curve_v2",
            "margin": args.margin,
            "worlds": args.worlds,
            "rule": {"keep": args.keep, "z": args.z, "tie": args.tie,
                "halves": "tables of an even seed choose the leaf, tables of an odd seed report; \
                           every deal of a table is in one half",
                "standard_error": "by game, the ratio estimator's; a game is a games file and deal",
                "population": "priority roots; combat declarations informational"},
            "roots": self.placed.len(),
            "events_left_out": self.events,
            "leaves_left_out": self.leaves,
            "selection": selection,
            "report": report,
            "chosen": verdict,
            "informational_combat": table(&combat),
            "informational_all_priority": table(&every),
        })
    }
}

/// Some roots' table: the full lead and each leaf's lead, share and z.
fn table(roots: &[&Placed]) -> Value {
    let full_of = |roots: &[&Placed]| {
        let mut leads = Leads::default();
        for placed in roots {
            leads.add(
                placed.game,
                placed.root.full.replaced,
                placed.root.full.lead,
            );
        }
        leads
    };
    let mut leaves = serde_json::Map::new();
    for name in LEAVES {
        let judged: Vec<&Placed> = roots
            .iter()
            .copied()
            .filter(|placed| placed.root.leaves.contains_key(name))
            .collect();
        if judged.is_empty() {
            continue;
        }
        let mut leads = Leads::default();
        let (mut agree, mut steps) = (0_u64, 0_u64);
        for placed in &judged {
            let leaf = &placed.root.leaves[name];
            leads.add(placed.game, leaf.judged.replaced, leaf.judged.lead);
            agree += u64::from(leaf.agrees);
            steps += leaf.steps;
        }
        let (lead, se) = (leads.per_hundred().0, leads.per_hundred_se_by_game());
        let full = full_of(&judged).per_hundred().0;
        let count = real(judged.len() as u64);
        leaves.insert(
            name.to_owned(),
            json!({
                "roots": judged.len(),
                "lead": leads.report(),
                "z": if se > 0.0 { lead / se } else { 0.0 },
                "full_per_100": full,
                "kept": (full > 0.0).then(|| lead / full),
                "same_choices_as_the_net": real(agree) / count,
                "rollout_steps_per_root": real(steps) / count,
            }),
        );
    }
    json!({"roots": roots.len(), "full": full_of(roots).report(), "leaves": leaves})
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let mut roots = Roots::default();
    for (file, path) in args.games.iter().enumerate() {
        roots.file(file, path, &args)?;
    }
    let text = serde_json::to_string_pretty(&roots.report(&args))?;
    println!("{text}");
    if let Some(out) = &args.out {
        std::fs::write(out, text + "\n").with_context(|| format!("{}", out.display()))?;
    }
    roots.ensure_judged()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(own: &[[f64; 3]], leaf: &[[f64; 3]]) -> Value {
        let candidates =
            json!(["PassPriority", {"PlayLand": {"card": 1}}, {"PlayLand": {"card": 2}}]);
        json!({"candidates": candidates, "worlds": own, "accepted": true,
            "variants": {"jump": {"candidates": candidates, "worlds": leaf, "rollout_steps": 40}}})
    }

    /// The net: candidate 2 leads by 0.1 in every world.
    const OWN: [[f64; 3]; 4] = [[0.5, 0.5, 0.6]; 4];

    fn args(extra: &[&str]) -> Args {
        let mut argv = vec!["rollout_curve4", "--games", "g"];
        argv.extend_from_slice(extra);
        Args::try_parse_from(argv).unwrap()
    }

    fn game(i: u64, seed: u64, events: Vec<Value>) -> Game {
        Game {
            i,
            seed,
            search_events: events,
        }
    }

    #[test]
    fn a_leaf_is_judged_by_the_nets_worlds_not_its_own() {
        // The leaf's own worlds prefer candidate 1, which the net scores even.
        let leaf = [[0.5, 0.7, 0.5]; 4];
        let root = root(&event(&OWN, &leaf), 4, 0.03).unwrap();
        assert!((root.full.lead - 0.1).abs() < 1e-12);
        let jump = &root.leaves["jump"];
        assert!(
            jump.judged.lead.abs() < 1e-12,
            "judged by the net, it gains nothing"
        );
        assert!(jump.judged.replaced && !jump.agrees);
        assert!(!root.leaves.contains_key("follow"), "not compared");
        let same = super::root(&event(&OWN, &OWN), 4, 0.03).unwrap();
        assert!((same.leaves["jump"].judged.lead - 0.1).abs() < 1e-12);
        assert!(same.leaves["jump"].agrees);
    }

    /// What keeps a whole event out, and what keeps one leaf out of it.
    #[test]
    fn an_event_or_one_leaf_is_left_out_by_name() {
        let why = |event: &Value| root(event, 4, 0.03).err();
        let mut refused = event(&OWN, &OWN);
        refused["accepted"] = json!(false);
        assert_eq!(why(&refused), Some("refused_by_the_table"));
        let mut short = event(&OWN, &OWN);
        short["worlds"] = json!([[0.5, 0.5, 0.6]]);
        assert_eq!(why(&short), Some("incomplete_worlds"));
        let mut narrow = event(&OWN, &OWN);
        narrow["worlds"] = json!([[0.5, 0.5], [0.5, 0.5], [0.5, 0.5], [0.5, 0.5]]);
        assert_eq!(why(&narrow), Some("incomplete_worlds"));
        let mut none = event(&OWN, &OWN);
        none["worlds"] = Value::Null;
        assert_eq!(why(&none), Some("no_worlds"));
        assert_eq!(why(&json!({"worlds": OWN})), Some("no_candidates"));

        let mut mixed = event(&OWN, &OWN);
        mixed["variants"]["student"] = json!({"unsearched": "Skipped(NoAlternative)"});
        mixed["variants"]["follow"] = mixed["variants"]["jump"].clone();
        mixed["variants"]["follow"]["candidates"] = json!(["PassPriority"]);
        let judged = root(&mixed, 4, 0.03).unwrap();
        assert!(judged.leaves.keys().eq(&["jump"]));
        assert_eq!(judged.left_out["student"], "unsearched");
        assert_eq!(judged.left_out["follow"], "other_candidates");
        mixed["variants"]["jump"]["worlds"] = json!([[0.5, 0.5, 0.6]]);
        let judged = root(&mixed, 4, 0.03).unwrap();
        assert_eq!(judged.left_out["jump"], "incomplete_worlds");
    }

    /// Both deals of a table, one seating the net in each chair, are in one
    /// half; a deal number means nothing across files.
    #[test]
    fn a_table_is_in_one_half_and_a_game_is_a_file_and_deal() {
        let args = args(&[]);
        let mut roots = Roots::default();
        for (file, i, seed) in [(0, 0, 10), (0, 1, 10), (0, 2, 11), (0, 3, 11), (1, 0, 10)] {
            let events = vec![event(&OWN, &OWN); 2];
            roots.game(file, &game(i, seed, events), &args);
        }
        let halves: Vec<bool> = roots.placed.iter().map(|p| p.selection).collect();
        assert_eq!(
            halves,
            [
                true, true, true, true, false, false, false, false, true, true
            ]
        );
        let report = roots.report(&args);
        assert_eq!(report["contract"], "rollout_net_curve_v2");
        assert_eq!(report["selection"]["full"]["games"], 3);
        assert_eq!(report["report"]["full"]["games"], 2);
        assert_eq!(report["report"]["leaves"]["jump"]["lead"]["games"], 2);
        assert_eq!(report["chosen"]["leaf"], "jump");
        assert_eq!(report["chosen"]["kept"], 1.0);
        assert_eq!(report["chosen"]["pass"], false, "no spread, no z");
    }

    /// The error by game is the ratio estimator's: games of one lead a root
    /// have none, however many roots each has.
    #[test]
    fn the_error_by_game_is_the_ratio_estimators() {
        let args = args(&[]);
        let mut roots = Roots::default();
        roots.game(0, &game(0, 2, vec![event(&OWN, &OWN); 9]), &args);
        roots.game(0, &game(1, 2, vec![event(&OWN, &OWN)]), &args);
        let full = &roots.report(&args)["selection"]["full"];
        assert!((full["lead_per_100"].as_f64().unwrap() - 10.0).abs() < 1e-9);
        assert!(full["lead_per_100_se_by_game"].as_f64().unwrap().abs() < 1e-9);
        assert_eq!(full["decisions"], 10);
    }

    /// A run of another P is read as such, and what was left out is named.
    #[test]
    fn the_worlds_of_a_complete_root_are_the_runs() {
        let six = [[0.5, 0.5, 0.6]; 6];
        let mut roots = Roots::default();
        let events = vec![event(&six, &six), event(&OWN, &OWN)];
        roots.game(0, &game(0, 2, events), &args(&["--worlds", "6"]));
        assert_eq!(roots.placed.len(), 1);
        assert_eq!(roots.events["incomplete_worlds"], 1);
        let report = roots.report(&args(&["--worlds", "6"]));
        assert_eq!(report["events_left_out"]["incomplete_worlds"], 1);
        assert_eq!(report["worlds"], 6);
        assert!(Args::try_parse_from(["rollout_curve4", "--games", "g", "--worlds", "1"]).is_err());
    }

    /// A run of which nothing is left is an error that says what was left out.
    #[test]
    fn nothing_left_to_judge_is_an_error() {
        let args = args(&[]);
        let mut roots = Roots::default();
        assert!(roots.ensure_judged().is_err(), "no event at all");
        let mut refused = event(&OWN, &OWN);
        refused["accepted"] = json!(false);
        let mut unsearched = event(&OWN, &OWN);
        unsearched["variants"]["jump"] = json!({"unsearched": "Skipped(NoAlternative)"});
        roots.game(0, &game(0, 2, vec![refused, unsearched]), &args);
        let error = roots.ensure_judged().unwrap_err().to_string();
        assert!(
            error.contains("refused_by_the_table") && error.contains("unsearched"),
            "{error}"
        );
        assert!(roots.report(&args)["chosen"].is_null());
        roots.game(0, &game(1, 2, vec![event(&OWN, &OWN)]), &args);
        roots.ensure_judged().unwrap();
    }
}
