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
//! Each level needs the one below. L4 and L5 read files the engine's test
//! hooks write (`docs/verification-hooks.md`) and stay empty until they
//! exist. A card named in `--demote` (an open bug report) stops at L1
//! whatever its evidence. The trained AI trains on L4 cards once L4 exists;
//! until then the population per level says which tests the pool is
//! missing.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::Context as _;
use baylee_core::ids::CardIndex;
use baylee_train::working::Working;

/// A card's level: 0 when not even implemented.
type Level = u8;

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
pub fn verify(root: &Path, out: &Path, demote: Option<&Path>) -> anyhow::Result<()> {
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
    let demoted = demote.map(demoted).transpose()?.unwrap_or_default();

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
    let mut deck_files: Vec<_> = fs::read_dir(root.join("data/decks"))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "txt"))
        .collect();
    deck_files.sort();
    for path in deck_files {
        let deck = baylee_train::housedeck::HouseDeck::load(&path)?;
        let cards = deck.distinct();
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
        "cards": cards_json,
    });
    if let Some(dir) = out.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(out, serde_json::to_vec_pretty(&report)?)?;
    println!("report: {}", out.display());
    Ok(())
}
