//! `xtask verify`: how far each card is verified, as a ladder of evidence.
//!
//! | Level | Evidence |
//! |---|---|
//! | L1 | `Coverage::Implemented`: every clause of the card was read |
//! | L2 | and `validate` reports nothing for its file (oracle text, types, printed cost) |
//! | L3 | and the engine's test code names it (`baylee_train::working`) |
//! | L4 | and every ability of it fired in a test, every mechanic it uses is tested, and it leaves the battlefield clean |
//! | L5 | and removing any one of its abilities makes one of its tests fail |
//!
//! Each level needs the one below. L4 reads a coverage export of the
//! engine's rule tests (`--coverage`, [`crate::mechanics`]), the firing
//! recorder's directory (`--ability-log`, [`crate::hooks`]) and the leave
//! probe's (`--leave-log`); L5 runs one mutant per ability (`--mutate`). All
//! follow `docs/verification-hooks.md`. Without `--leave-log` L4 is granted
//! without its leave part, and the report says `leave_checked: false`. A card named in `--demote` (an
//! open bug report) stops at L1 whatever its evidence. The trained AI trains
//! on L4 cards; the population per level says which tests the pool is
//! missing.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Context as _;
use baylee_core::ids::CardIndex;
use baylee_train::working::Working;

/// A card's level: 0 when not even implemented.
type Level = u8;

/// What `verify` reads besides the pool, and where it writes.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Where the per-card report is written.
    #[arg(long, default_value = "target/verify.json")]
    out: PathBuf,
    /// Cards an open bug report names, one name or index per line; each
    /// stops at L1.
    #[arg(long)]
    demote: Option<PathBuf>,
    /// An llvm-cov JSON export of the engine's rule tests, for L4's
    /// mechanics part (`xtask/src/mechanics.rs` says how to make one).
    #[arg(long)]
    coverage: Option<PathBuf>,
    /// The directory a run of the engine's tests with `BAYLEE_ABILITY_LOG`
    /// wrote, for L4's firing part (`docs/verification-hooks.md`).
    #[arg(long)]
    ability_log: Option<PathBuf>,
    /// The directory the leave probe wrote `leave.jsonl` into
    /// (`BAYLEE_LEAVE_LOG`), for L4's leave part.
    #[arg(long)]
    leave_log: Option<PathBuf>,
    /// Run L5: one mutant per ability of every L4 card, against the tests
    /// that fired the card.
    #[arg(long)]
    mutate: bool,
    /// Threads for the mutants; 0 = one per core.
    #[arg(long, default_value_t = 0)]
    threads: usize,
}

/// What `validate` reported: per card file slug, and about no one card.
type Findings = (BTreeMap<String, Vec<String>>, Vec<String>);

/// The problems `validate` reports, by card file slug, and the ones it
/// reports about no one card.
fn validate_findings(root: &Path) -> anyhow::Result<Findings> {
    let exe = std::env::current_exe().context("the xtask binary")?;
    let output = Command::new(exe)
        .arg("validate")
        .current_dir(root)
        .output()
        .context("running `xtask validate`")?;
    let text = String::from_utf8_lossy(&output.stdout);
    let slugs: BTreeSet<String> = baylee_cards::all()
        .map(|d| super::front_face_slug(d.name()))
        .collect();
    let mut by_slug: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut global = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        let slug = line
            .strip_prefix("MISSING FILE: ")
            .or_else(|| line.split_once(':').map(|(s, _)| s))
            .map(str::trim);
        match slug {
            Some(s) if slugs.contains(s) => by_slug
                .entry(s.to_owned())
                .or_default()
                .push(line.to_owned()),
            _ if line.contains("problem") || line.starts_with("only ") => {
                global.push(line.to_owned());
            }
            _ => {}
        }
    }
    Ok((by_slug, global))
}

/// Cards named in `path`, one card name or `CardIndex` per line; `#` starts
/// a comment.
fn demoted(path: &Path) -> anyhow::Result<BTreeSet<CardIndex>> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let mut out = BTreeSet::new();
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let card = line
            .parse::<u32>()
            .ok()
            .map(CardIndex::new)
            .or_else(|| baylee_cards::decks::by_name(line))
            .with_context(|| format!("no card {line:?} in the pool"))?;
        out.insert(card);
    }
    Ok(out)
}

/// Runs the ladder over the pool and every house deck.
///
/// # Errors
/// When the tests or a deck cannot be read, or `validate` cannot run.
#[allow(clippy::too_many_lines)] // one ladder, then its three reports
pub fn verify(root: &Path, inputs: &Args) -> anyhow::Result<()> {
    let out = &root.join(&inputs.out);
    let working = Working::scan(root).context("reading the engine's test code")?;
    let cache = root.join("data/scryfall-cache");
    let l2_checked = cache.is_dir();
    let (findings, global) = if l2_checked {
        validate_findings(root)?
    } else {
        println!(
            "L2 not checked: no data/scryfall-cache (`cargo run -p xtask -- scryfall-cache` fills it)"
        );
        (BTreeMap::new(), Vec::new())
    };
    let demoted = inputs
        .demote
        .as_deref()
        .map(demoted)
        .transpose()?
        .unwrap_or_default();

    // A card's level, and the evidence it stops at.
    let level_of = |def: &baylee_cards::dsl::CardDef| -> (Level, String) {
        if !def.is_implemented() {
            return (0, "not implemented".into());
        }
        if demoted.contains(&def.index) {
            return (1, "demoted: an open bug report names it".into());
        }
        if !l2_checked {
            return (1, "L2 not checked".into());
        }
        if let Some(problems) = findings.get(&super::front_face_slug(def.name())) {
            return (1, problems.first().cloned().unwrap_or_default());
        }
        if !working.tested.contains_key(&def.index) {
            return (2, "no engine test names it".into());
        }
        (3, "L4 waits for the engine's test hooks".into())
    };
    let mut levels: BTreeMap<CardIndex, Level> = BTreeMap::new();
    let mut why: BTreeMap<CardIndex, String> = BTreeMap::new();
    for def in baylee_cards::all() {
        let (level, stop) = level_of(def);
        levels.insert(def.index, level);
        why.insert(def.index, stop);
    }

    let analysis = inputs
        .coverage
        .as_deref()
        .map(|c| crate::mechanics::analyse(root, c))
        .transpose()
        .context("reading the engine's mechanics coverage")?;
    let firing = inputs
        .ability_log
        .as_deref()
        .map(crate::hooks::Firing::read)
        .transpose()
        .context("reading the firing recorder's directory")?;
    let leave = inputs
        .leave_log
        .as_deref()
        .map(crate::hooks::Leave::read)
        .transpose()
        .context("reading the leave probe's directory")?;
    // L4: no untested mechanic, every ability fired, and (where the probe
    // ran) the battlefield left clean.
    for (card, level) in &mut levels {
        if *level != 3 {
            continue;
        }
        let mechanics = analysis.as_ref().map(|a| a.stop(*card));
        let fired = firing.as_ref().map(|f| f.gap(*card, analysis.as_ref()));
        let left = leave.as_ref().and_then(|l| l.gap(*card));
        let stop = match (mechanics, fired, left) {
            (Some(Some(stop)), _, _)
            | (_, Some(Some(stop)), _)
            | (Some(None), Some(None), Some(stop)) => stop,
            (Some(None), Some(None), None) => {
                *level = 4;
                if leave.is_some() {
                    "L4".into()
                } else {
                    "L4 without its leave part (no --leave-log)".into()
                }
            }
            (None, _, _) => "L4 needs --coverage".into(),
            (_, None, _) => "L4 needs --ability-log".into(),
        };
        why.insert(*card, stop);
    }
    // L5: every mutant of an L4 card killed by the card's tests.
    let mut mutants_json = Vec::new();
    if inputs.mutate {
        let Some(firing) = &firing else {
            anyhow::bail!(
                "--mutate needs --ability-log: the tests to run are the ones that fired the card"
            );
        };
        let l4: BTreeSet<CardIndex> = levels
            .iter()
            .filter(|(_, l)| **l == 4)
            .map(|(c, _)| *c)
            .collect();
        let work = out.with_file_name("mutants");
        let verdicts = crate::hooks::mutate(root, firing, &l4, inputs.threads, &work)?;
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        for ((card, index), verdict) in &verdicts {
            *counts.entry(verdict.name()).or_default() += 1;
            let reason = match verdict {
                crate::hooks::Verdict::Invalid(why) => Some(why.clone()),
                _ => None,
            };
            mutants_json.push(serde_json::json!({
                "card": card.get(), "index": index, "verdict": verdict.name(), "why": reason,
            }));
        }
        println!("L5 mutants: {counts:?}");
        for card in &l4 {
            let survivors: Vec<String> = verdicts
                .iter()
                .filter(|((c, _), v)| c == card && !v.killed())
                .map(|((_, i), v)| format!("{i} {}", v.name()))
                .collect();
            if survivors.is_empty() {
                levels.insert(*card, 5);
                why.insert(*card, "L5".into());
            } else {
                why.insert(
                    *card,
                    format!("mutants not killed: {}", survivors.join(", ")),
                );
            }
        }
    }

    let population = |cards: &BTreeSet<CardIndex>| -> [usize; 6] {
        let mut p = [0_usize; 6];
        for c in cards {
            p[usize::from(levels.get(c).copied().unwrap_or(0))] += 1;
        }
        p
    };
    let pool: BTreeSet<CardIndex> = levels.keys().copied().collect();
    let print_row = |name: &str, p: [usize; 6]| {
        let total: usize = p.iter().sum();
        let at_least = |l: usize| p[l..].iter().sum::<usize>();
        println!(
            "{name:28} {total:5}   L1+ {:5}   L2+ {:5}   L3+ {:5}   L4+ {:5}   L5 {:5}",
            at_least(1),
            at_least(2),
            at_least(3),
            at_least(4),
            p[5]
        );
    };
    println!("{:28} {:>5}   (cards at or above each level)", "", "cards");
    print_row("pool", population(&pool));
    let mut decks_json = Vec::new();
    let mut house = BTreeSet::new();
    let mut deck_files: Vec<_> = fs::read_dir(root.join("data/decks"))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "txt"))
        .collect();
    deck_files.sort();
    for path in deck_files {
        let deck = baylee_train::housedeck::HouseDeck::load(&path)?;
        let cards = deck.distinct();
        house.extend(cards.iter().copied());
        let p = population(&cards);
        print_row(&format!("deck {}", deck.key), p);
        let below: Vec<String> = cards
            .iter()
            .filter(|c| levels.get(c).copied().unwrap_or(0) < 3)
            .map(|c| {
                baylee_cards::by_index(*c).map_or_else(|| c.to_string(), |d| d.name().to_owned())
            })
            .collect();
        decks_json.push(serde_json::json!({"deck": deck.key, "population": p, "below_l3": below}));
    }
    if !global.is_empty() {
        println!(
            "validate reported {} problem(s) about no one card:",
            global.len()
        );
        for g in global.iter().take(10) {
            println!("  {g}");
        }
    }
    let mechanics_json = analysis.as_ref().map(|a| {
        let l3: BTreeSet<CardIndex> = levels
            .iter()
            .filter(|(_, l)| **l >= 3)
            .map(|(c, _)| *c)
            .collect();
        a.report(&l3, &house)
    });
    if let Some(json) = &mechanics_json {
        let path = out.with_file_name("mechanics.json");
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::write(&path, serde_json::to_vec_pretty(json)?)?;
        println!("mechanics report: {}", path.display());
    }
    let cards_json: Vec<_> = levels
        .iter()
        .map(|(c, l)| {
            serde_json::json!({
                "card": c.get(),
                "name": baylee_cards::by_index(*c).map(baylee_cards::dsl::CardDef::name),
                "level": l,
                "stops_at": why.get(c),
            })
        })
        .collect();
    let report = serde_json::json!({
        "ladder": ["L1 implemented", "L2 validate clean", "L3 named by engine tests", "L4 abilities fired + mechanics tested + leaves clean", "L5 mutation-killed"],
        "l2_checked": l2_checked,
        "working_hash": working.hash(),
        "pool": population(&pool),
        "decks": decks_json,
        "validate_global": global,
        "mechanics_checked": mechanics_json.is_some(),
        "firing_checked": firing.is_some(),
        "leave_checked": leave.is_some(),
        "l5_checked": inputs.mutate,
        "firing": firing.as_ref().map(|f| serde_json::json!({
            "test_files": f.files,
            "inventory_cards": f.inventory.len(),
            "fired_entries": f.fired.len(),
            "outside_inventory": f.outside.iter().map(|(c, i, k)| serde_json::json!([c.get(), i, k])).collect::<Vec<_>>(),
        })),
        "mutants": mutants_json,
        "cards": cards_json,
    });
    if let Some(dir) = out.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(out, serde_json::to_vec_pretty(&report)?)?;
    println!("report: {}", out.display());
    Ok(())
}
