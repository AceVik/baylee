//! Offline deck corpus importer, validator, deduplicator, and split manifest
//! builder (`import_decks`) for Playernet v4 (§12–§16 of
//! `docs/history/trained-ai-v4-argon-feedback-2026-10-22.md`).
//!
//! Uses existing parser and card-identity infrastructure (`HouseDeck::parse_any`,
//! `baylee_deckio`, `baylee_cards::decks::by_name`, `baylee_cards::formats::check`,
//! `baylee_train::working::Working`) and enforces:
//! - Zero basic-land substitution for broken/missing cards;
//! - Explicit separation between unmodified original decks (`is_derived: false`)
//!   and rule-validated adapted precon derivations (`is_derived: true` with
//!   `original_ref` and per-card `adaptations` changelog);
//! - Preservation of rejected precon lists outside the active pool with exact
//!   refusal reasons (`/home/ace/baylee-data/v4/decks/rejected.v4.json`);
//! - Canonical card-multiset Jaccard deduplication (`J < 0.88` within pool and
//!   `J <= 0.85` across `train` vs `val`/`test` splits);
//! - Strict protection of the 5% zero-shot card holdout (`deckgen::held_out(&working, 20)`):
//!   any deck containing a held-out card is routed to `split = "test"`
//!   (`holdout_tier = "unseen_cards"`), never `train` or `val`.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context as _, bail};
use baylee_cards::decks::{leader_of, may_lead_together};
use baylee_cards::formats::{self, Shape};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_train::deckgen::strategy::MultisetSimilarityIndex;
use baylee_train::deckgen::{self, Archetype, CoverageConfig, Rng, build_coverage_corpus_scoped};
use baylee_train::housedeck::HouseDeck;
use baylee_train::working::{Refusal, Working, repo_root};
use clap::Parser;
use serde::{Deserialize, Serialize};

const IMPORTER_VERSION: &str = "v4.2.0-2026-10-25";

#[derive(Parser, Debug)]
#[command(about = "Imports, validates, deduplicates, and splits the Playernet v4 deck corpus")]
struct Args {
    /// Output path for the active v4 deck manifest JSON.
    #[arg(long, default_value = "data/decks/manifest.v4.json")]
    manifest_out: PathBuf,

    /// Directory where imported/adapted/curated `.txt` deck files are written.
    #[arg(long, default_value = "data/decks/imported")]
    imported_dir: PathBuf,

    /// Output path for the rejected-decks provenance log.
    #[arg(
        long,
        default_value = "/home/ace/baylee-data/v4/decks/rejected.v4.json"
    )]
    rejected_out: PathBuf,

    /// Output path for the human-readable corpus report.
    #[arg(long, default_value = "docs/history/deck-corpus-v4-2026-10-22.md")]
    report_out: PathBuf,

    /// Maximum Jaccard similarity allowed between any two accepted decks.
    #[arg(long, default_value_t = 0.88)]
    max_within_jaccard: f64,

    /// Maximum Jaccard similarity allowed across `train` vs `val`/`test` splits.
    #[arg(long, default_value_t = 0.85)]
    max_cross_split_jaccard: f64,

    /// Minimum number of decks each working card must appear in (`>= 3`; `0` to skip).
    #[arg(long, default_value_t = 3)]
    min_card_coverage: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CardAdaptation {
    removed_card: String,
    added_card: String,
    copies: u32,
    reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "Independent properties in the existing deck manifest schema"
)]
struct ManifestDeckEntry {
    id: String,
    name: String,
    path: String,
    format: String,
    shape_valid: bool,
    archetype: String,
    strategy_tags: Vec<String>,
    strategy_confidence: &'static str,
    colors: String,
    source_key: String,
    source_ref: String,
    source_date: String,
    license: String,
    is_derived: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    original_ref: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    adaptations: Vec<CardAdaptation>,
    main_count: usize,
    sideboard_count: usize,
    sideboard_supported: bool,
    commanders: Vec<String>,
    distinct_cards: usize,
    contains_held_out_cards: bool,
    held_out_card_count: usize,
    deck_hash: String,
    similarity_group_id: usize,
    max_cross_split_jaccard: f64,
    holdout_tier: String,
    split: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct RejectedDeckEntry {
    key: String,
    name: String,
    source_ref: String,
    kind: String,
    total_cards: usize,
    failing_distinct_cards: usize,
    reasons: BTreeMap<String, usize>,
    sample_failing_cards: Vec<String>,
}

#[derive(Clone, Debug)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "Independent validation and provenance properties mirror the manifest"
)]
struct CandidateDeck {
    id: String,
    name: String,
    rel_path: String,
    format: String,
    shape_valid: bool,
    archetype: String,
    strategy_tags: Vec<String>,
    strategy_confidence: &'static str,
    colors: String,
    source_key: String,
    source_ref: String,
    source_date: String,
    license: String,
    is_derived: bool,
    original_ref: Option<String>,
    adaptations: Vec<CardAdaptation>,
    main_cards: Vec<CardIndex>,
    side_cards: Vec<CardIndex>,
    sideboard_supported: bool,
    commander_names: Vec<String>,
    multiset: BTreeMap<CardIndex, u32>,
    contains_held_out_cards: bool,
    held_out_card_count: usize,
    deck_hash: String,
    forced_split: Option<(String, String)>,
}

mod document;
mod replacement;
mod similarity;
mod strategy;
use document::format_house_deck_text;
use replacement::find_functional_replacement;
use similarity::{compute_deck_hash, compute_multiset, find, multiset_jaccard};
use strategy::{classify_deck_strategy, color_string};

#[allow(clippy::too_many_lines)]
fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let root = repo_root();
    let working = Working::scan(&root).context("scanning Working card pool")?;
    let held_out_set = deckgen::held_out(&working, 20);

    let train_safe_working: Vec<CardIndex> = working
        .cards
        .iter()
        .copied()
        .filter(|c| !held_out_set.contains(c))
        .collect();

    println!(
        "[import_decks] Pool: {} working cards ({} train-safe, {} held-out 5% zero-shot slice)",
        working.cards.len(),
        train_safe_working.len(),
        held_out_set.len()
    );

    let imported_root = root.join(&args.imported_dir);
    fs::create_dir_all(imported_root.join("precon"))?;
    fs::create_dir_all(imported_root.join("adapted"))?;
    fs::create_dir_all(imported_root.join("curated"))?;
    if let Some(parent) = args.rejected_out.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut candidates: Vec<CandidateDeck> = Vec::new();
    let mut rejected: Vec<RejectedDeckEntry> = Vec::new();

    // -------------------------------------------------------------------------
    // 1. Load the 15 existing curated evaluation benchmark decks (`data/decks/eval/*.txt`)
    // -------------------------------------------------------------------------
    let eval_dir = root.join("data/decks/eval");
    if eval_dir.exists() {
        let mut eval_files: Vec<PathBuf> = fs::read_dir(&eval_dir)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|ext| ext == "txt"))
            .collect();
        eval_files.sort();
        for path in eval_files {
            let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
            let text = fs::read_to_string(&path)?;
            let hd = HouseDeck::parse_any(&stem, &text)?;
            let failing = hd.failing(&working);
            if !failing.is_empty() {
                bail!("Eval deck {stem} has failing cards: {failing:?}");
            }
            let main_cards: Vec<CardIndex> = hd.deck.main.iter().map(|c| c.index).collect();
            let side_cards: Vec<CardIndex> = hd.deck.sideboard.iter().map(|c| c.index).collect();
            let cmd_cards: Vec<CardIndex> = hd.deck.commanders.iter().map(|c| c.index).collect();
            let cmd_names: Vec<String> = cmd_cards
                .iter()
                .filter_map(|&c| baylee_cards::by_index(c).map(|d| d.name().to_string()))
                .collect();
            let fmt = if cmd_cards.is_empty() {
                "constructed"
            } else {
                "commander"
            };
            let shape = if cmd_cards.is_empty() {
                Shape::Constructed
            } else {
                Shape::Commander
            };
            let shape_valid = formats::check(shape, &main_cards, &cmd_cards).is_empty();
            let (arch, tags, colors) = classify_deck_strategy(&main_cards, &cmd_cards);
            let multiset = compute_multiset(&main_cards, &cmd_cards);
            let held_cnt = multiset.keys().filter(|c| held_out_set.contains(c)).count();
            let deck_hash = compute_deck_hash(&multiset, &cmd_cards);
            let rel_path = path
                .strip_prefix(&root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");

            candidates.push(CandidateDeck {
                id: format!("eval/{stem}"),
                name: hd.deck.name.clone(),
                rel_path,
                format: fmt.to_string(),
                shape_valid,
                archetype: arch,
                strategy_tags: tags,
                strategy_confidence: "curated_exact",
                colors: color_string(colors),
                source_key: "baylee_eval".to_string(),
                source_ref: format!("data/decks/eval/{stem}.txt"),
                source_date: "2026-10-22".to_string(),
                license: "AGPL-3.0-only".to_string(),
                is_derived: false,
                original_ref: None,
                adaptations: Vec::new(),
                main_cards,
                side_cards,
                sideboard_supported: true,
                commander_names: cmd_names,
                multiset,
                contains_held_out_cards: held_cnt > 0,
                held_out_card_count: held_cnt,
                deck_hash,
                forced_split: Some(("test".to_string(), "eval_benchmark".to_string())),
            });
        }
    }

    // -------------------------------------------------------------------------
    // 2. Load the 9 house decks (`data/decks/*.txt`)
    // -------------------------------------------------------------------------
    let house_dir = root.join("data/decks");
    let mut house_files: Vec<PathBuf> = fs::read_dir(&house_dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && p.extension().is_some_and(|ext| ext == "txt"))
        .collect();
    house_files.sort();
    for path in house_files {
        let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
        let text = fs::read_to_string(&path)?;
        let Ok(hd) = HouseDeck::parse_any(&stem, &text) else {
            continue;
        };
        let failing = hd.failing(&working);
        if !failing.is_empty() {
            continue;
        }
        let main_cards: Vec<CardIndex> = hd.deck.main.iter().map(|c| c.index).collect();
        let side_cards: Vec<CardIndex> = hd.deck.sideboard.iter().map(|c| c.index).collect();
        let cmd_cards: Vec<CardIndex> = hd.deck.commanders.iter().map(|c| c.index).collect();
        let cmd_names: Vec<String> = cmd_cards
            .iter()
            .filter_map(|&c| baylee_cards::by_index(c).map(|d| d.name().to_string()))
            .collect();
        let fmt = if cmd_cards.is_empty() {
            "constructed"
        } else {
            "commander"
        };
        let shape = if cmd_cards.is_empty() {
            Shape::Constructed
        } else {
            Shape::Commander
        };
        let shape_valid = formats::check(shape, &main_cards, &cmd_cards).is_empty();
        let (arch, tags, colors) = classify_deck_strategy(&main_cards, &cmd_cards);
        let multiset = compute_multiset(&main_cards, &cmd_cards);
        let held_cnt = multiset.keys().filter(|c| held_out_set.contains(c)).count();
        let deck_hash = compute_deck_hash(&multiset, &cmd_cards);
        let rel_path = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let forced = if held_cnt > 0 {
            Some(("test".to_string(), "unseen_cards".to_string()))
        } else {
            None
        };
        candidates.push(CandidateDeck {
            id: format!("house/{stem}"),
            name: hd.deck.name.clone(),
            rel_path,
            format: fmt.to_string(),
            shape_valid,
            archetype: arch,
            strategy_tags: tags,
            strategy_confidence: "curated_exact",
            colors: color_string(colors),
            source_key: "baylee_house".to_string(),
            source_ref: format!("data/decks/{stem}.txt"),
            source_date: "2026-10-22".to_string(),
            license: "AGPL-3.0-only".to_string(),
            is_derived: false,
            original_ref: None,
            adaptations: Vec::new(),
            main_cards,
            side_cards,
            sideboard_supported: true,
            commander_names: cmd_names,
            multiset,
            contains_held_out_cards: held_cnt > 0,
            held_out_card_count: held_cnt,
            deck_hash,
            forced_split: forced,
        });
    }

    // -------------------------------------------------------------------------
    // 3. Scan all 1,203 MTGJSON retail precon decks (`data/decks/precon/<set>/<slug>.txt`)
    //    - Accept 100% Working unmodified retail decks (`is_derived: false`)
    //    - Record rejected retail decks with exact refusal reasons
    //    - Create rule-validated derived adaptations (`is_derived: true`) for near-complete
    //      60-card Constructed and 100-card Commander retail decks with full changelog
    // -------------------------------------------------------------------------
    let precon_root = root.join("data/decks/precon");
    let mut set_dirs: Vec<PathBuf> = fs::read_dir(&precon_root)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .collect();
    set_dirs.sort();

    for set_dir in set_dirs {
        let mut deck_files: Vec<PathBuf> = fs::read_dir(&set_dir)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|ext| ext == "txt"))
            .collect();
        deck_files.sort();

        for path in deck_files {
            let rel_precon = path
                .strip_prefix(&precon_root)
                .unwrap_or(&path)
                .with_extension("")
                .to_string_lossy()
                .replace('\\', "/");
            let text = fs::read_to_string(&path)?;

            let mut meta_name = rel_precon.clone();
            let mut meta_source = String::new();
            let mut meta_kind = String::new();
            let mut meta_released = String::from("2026-09-29");
            for line in text.lines() {
                if let Some(v) = line.strip_prefix("# name: ") {
                    meta_name = v.trim().to_string();
                } else if let Some(v) = line.strip_prefix("# source: ") {
                    meta_source = v.trim().to_string();
                } else if let Some(v) = line.strip_prefix("# type: ") {
                    meta_kind = v.trim().to_string();
                } else if let Some(v) = line.strip_prefix("# released: ") {
                    meta_released = v.trim().to_string();
                }
            }

            let imported = baylee_deckio::import(&text)
                .map_err(|e| anyhow::anyhow!("reading {}: {e}", path.display()))?;
            let baylee_deckio::Import::Read { read, .. } = imported else {
                continue;
            };

            let mut main_ok: Vec<CardIndex> = Vec::new();
            let mut side_ok: Vec<CardIndex> = Vec::new();
            let mut cmd_ok: Vec<CardIndex> = Vec::new();
            let mut main_failing: Vec<(String, u32, String)> = Vec::new();
            let mut side_failing: Vec<(String, u32, String)> = Vec::new();
            let mut cmd_failing: Vec<(String, String)> = Vec::new();
            let mut reason_counts: BTreeMap<String, usize> = BTreeMap::new();
            let mut sample_failing: Vec<String> = Vec::new();
            let mut total_cards = 0_usize;

            for card in &read.document.cards {
                if card.zone == baylee_deckio::Zone::Maybe {
                    continue;
                }
                total_cards += card.count as usize;
                let check_res: Result<CardIndex, &'static str> =
                    match baylee_cards::decks::by_name(&card.name) {
                        Some(cid) => match working.check(cid) {
                            Ok(()) => Ok(cid),
                            Err(Refusal::NotInPool) => Err("not_in_pool"),
                            Err(Refusal::NotImplemented) => Err("stub_card"),
                            Err(Refusal::Untested) => Err("untested_card"),
                        },
                        None => {
                            if baylee_cards_index::row_by_name(&card.name).is_some() {
                                Err("not_in_pool")
                            } else {
                                Err("not_in_ledger")
                            }
                        }
                    };
                match (card.zone, check_res) {
                    (baylee_deckio::Zone::Main, Ok(cid)) => {
                        for _ in 0..card.count {
                            main_ok.push(cid);
                        }
                    }
                    (baylee_deckio::Zone::Side, Ok(cid)) => {
                        for _ in 0..card.count {
                            side_ok.push(cid);
                        }
                    }
                    (baylee_deckio::Zone::Commander, Ok(cid)) => {
                        if leader_of(cid).is_some_and(|l| l.eligible) {
                            cmd_ok.push(cid);
                        } else {
                            *reason_counts
                                .entry("invalid_leader".to_string())
                                .or_insert(0) += 1;
                            cmd_failing.push((card.name.clone(), "invalid_leader".to_string()));
                        }
                    }
                    (baylee_deckio::Zone::Maybe, Ok(_)) => {}
                    (zone, Err(why)) => {
                        *reason_counts.entry(why.to_string()).or_insert(0) += 1;
                        if sample_failing.len() < 6 && !sample_failing.contains(&card.name) {
                            sample_failing.push(format!("{} ({why})", card.name));
                        }
                        match zone {
                            baylee_deckio::Zone::Main => {
                                main_failing.push((card.name.clone(), card.count, why.to_string()));
                            }
                            baylee_deckio::Zone::Side => {
                                side_failing.push((card.name.clone(), card.count, why.to_string()));
                            }
                            baylee_deckio::Zone::Commander => {
                                cmd_failing.push((card.name.clone(), why.to_string()));
                            }
                            baylee_deckio::Zone::Maybe => {}
                        }
                    }
                }
            }
            if cmd_ok.len() == 2 {
                let l0 = leader_of(cmd_ok[0]);
                let l1 = leader_of(cmd_ok[1]);
                if !l0.zip(l1).is_some_and(|(a, b)| may_lead_together(&a, &b)) {
                    *reason_counts
                        .entry("cannot_lead_together".to_string())
                        .or_insert(0) += 1;
                    cmd_failing.push((
                        "commander_pair".to_string(),
                        "cannot_lead_together".to_string(),
                    ));
                }
            }

            // Case A: 100% of main + commanders pass Working!
            if main_failing.is_empty() && cmd_failing.is_empty() && !main_ok.is_empty() {
                let side_supported = side_failing.is_empty();
                let side_kept = if side_supported {
                    side_ok.clone()
                } else {
                    Vec::new()
                };
                let play_size = main_ok.len() + cmd_ok.len();
                let (fmt_name, shape_valid) = if !cmd_ok.is_empty() {
                    let ok = formats::check(Shape::Commander, &main_ok, &cmd_ok).is_empty();
                    ("commander".to_string(), ok)
                } else if play_size >= 60 {
                    let ok = formats::check(Shape::Constructed, &main_ok, &[]).is_empty();
                    ("constructed".to_string(), ok)
                } else {
                    ("starter_limited".to_string(), true)
                };

                let slug_id = rel_precon.replace('/', "-").to_ascii_lowercase();
                let norm_rel = format!("data/decks/imported/precon/{slug_id}.txt");
                let norm_text = format_house_deck_text(
                    &meta_name,
                    &meta_source,
                    "MIT (Copyright © 2018 – Present, Zach Halpern); see NOTICE",
                    &main_ok,
                    &side_kept,
                    &cmd_ok,
                );
                fs::write(root.join(&norm_rel), norm_text)?;

                let (arch, tags, colors) = classify_deck_strategy(&main_ok, &cmd_ok);
                let multiset = compute_multiset(&main_ok, &cmd_ok);
                let held_cnt = multiset.keys().filter(|c| held_out_set.contains(c)).count();
                let deck_hash = compute_deck_hash(&multiset, &cmd_ok);
                let cmd_names: Vec<String> = cmd_ok
                    .iter()
                    .filter_map(|&c| baylee_cards::by_index(c).map(|d| d.name().to_string()))
                    .collect();
                let forced = if held_cnt > 0 {
                    Some(("test".to_string(), "unseen_cards".to_string()))
                } else {
                    None
                };

                candidates.push(CandidateDeck {
                    id: format!("precon/{rel_precon}"),
                    name: meta_name,
                    rel_path: norm_rel,
                    format: fmt_name,
                    shape_valid,
                    archetype: arch,
                    strategy_tags: tags,
                    strategy_confidence: "heuristic_from_dsl",
                    colors: color_string(colors),
                    source_key: "mtgjson_precon".to_string(),
                    source_ref: format!("data/decks/precon/{rel_precon}.txt ({meta_source})"),
                    source_date: meta_released,
                    license: "MIT (MTGJSON 5.3.0)".to_string(),
                    is_derived: false,
                    original_ref: None,
                    adaptations: Vec::new(),
                    main_cards: main_ok,
                    side_cards: side_kept,
                    sideboard_supported: side_supported,
                    commander_names: cmd_names,
                    multiset,
                    contains_held_out_cards: held_cnt > 0,
                    held_out_card_count: held_cnt,
                    deck_hash,
                    forced_split: forced,
                });
                continue;
            }

            // Record rejected original deck outside the active pool (§14)
            let failing_distinct = main_failing.len() + side_failing.len() + cmd_failing.len();
            rejected.push(RejectedDeckEntry {
                key: rel_precon.clone(),
                name: meta_name.clone(),
                source_ref: format!("data/decks/precon/{rel_precon}.txt"),
                kind: meta_kind.clone(),
                total_cards,
                failing_distinct_cards: failing_distinct,
                reasons: reason_counts,
                sample_failing_cards: sample_failing,
            });

            // Case B: Create an explicitly marked derived adaptation (`is_derived: true`)
            // when the retail deck is a 60+ card Constructed deck (with <= 18 distinct failing
            // mainboard cards) or a 100-card Commander deck whose commander(s) work (with <= 35
            // distinct failing cards), replacing each failing card with a functionally matched
            // Working card (never substituting basic lands for spells!).
            let is_commander_product = !cmd_ok.is_empty() && cmd_failing.is_empty();
            let main_total_intended: usize = main_ok.len()
                + main_failing
                    .iter()
                    .map(|(_, c, _)| *c as usize)
                    .sum::<usize>();
            let can_adapt_constructed = cmd_ok.is_empty()
                && cmd_failing.is_empty()
                && main_total_intended >= 60
                && main_failing.len() <= 18;
            let can_adapt_commander = is_commander_product
                && (main_total_intended + cmd_ok.len() == 100)
                && main_failing.len() <= 35;

            if can_adapt_constructed || can_adapt_commander {
                let mut adapted_main: Vec<CardIndex> = main_ok
                    .iter()
                    .copied()
                    .filter(|c| !held_out_set.contains(c))
                    .collect();
                // Also replace any held-out card in the adapted deck so adapted precons can safely enter train/val
                let mut held_to_replace: BTreeMap<CardIndex, u32> = BTreeMap::new();
                for &c in &main_ok {
                    if held_out_set.contains(&c) {
                        *held_to_replace.entry(c).or_insert(0) += 1;
                    }
                }
                if cmd_ok.iter().any(|c| held_out_set.contains(c)) {
                    continue;
                }

                let mut deck_colors = ColorSet::EMPTY;
                if can_adapt_commander {
                    for &c in &cmd_ok {
                        if let Some(def) = baylee_cards::by_index(c) {
                            deck_colors = deck_colors.union(def.color_identity);
                        }
                    }
                } else {
                    for &c in &adapted_main {
                        if let Some(def) = baylee_cards::by_index(c) {
                            deck_colors = deck_colors.union(def.color_identity);
                        }
                    }
                    if deck_colors.is_empty() {
                        deck_colors = ColorSet::of(Color::Red).union(ColorSet::of(Color::Green));
                    }
                }

                let copy_limit: u32 = if can_adapt_commander { 1 } else { 4 };
                let mut counts = compute_multiset(&adapted_main, &cmd_ok);
                let mut adaptations: Vec<CardAdaptation> = Vec::new();
                let mut ok_all = true;

                for (h_cid, h_cnt) in held_to_replace {
                    let orig_name =
                        baylee_cards::by_index(h_cid).map_or("HeldOutCard", |d| d.name());
                    for _ in 0..h_cnt {
                        if let Some((rep_cid, rep_name)) = find_functional_replacement(
                            orig_name,
                            deck_colors,
                            copy_limit,
                            &counts,
                            &train_safe_working,
                        ) {
                            adapted_main.push(rep_cid);
                            *counts.entry(rep_cid).or_insert(0) += 1;
                            adaptations.push(CardAdaptation {
                                removed_card: orig_name.to_string(),
                                added_card: rep_name.to_string(),
                                copies: 1,
                                reason: "held_out_card_protection".to_string(),
                            });
                        } else {
                            ok_all = false;
                            break;
                        }
                    }
                }

                for (fail_name, fail_cnt, fail_why) in &main_failing {
                    if !ok_all {
                        break;
                    }
                    // Try replacing all copies with one card if copy_limit allows, or 1 by 1
                    let mut remaining = *fail_cnt;
                    while remaining > 0 {
                        let Some((rep_cid, rep_name)) = find_functional_replacement(
                            fail_name,
                            deck_colors,
                            copy_limit,
                            &counts,
                            &train_safe_working,
                        ) else {
                            ok_all = false;
                            break;
                        };
                        let cur = counts.get(&rep_cid).copied().unwrap_or(0);
                        let is_basic = formats::is_basic_land(rep_cid);
                        let add_n = if is_basic {
                            remaining
                        } else {
                            remaining.min(copy_limit.saturating_sub(cur)).max(1)
                        };
                        for _ in 0..add_n {
                            adapted_main.push(rep_cid);
                        }
                        *counts.entry(rep_cid).or_insert(0) += add_n;
                        adaptations.push(CardAdaptation {
                            removed_card: fail_name.clone(),
                            added_card: rep_name.to_string(),
                            copies: add_n,
                            reason: fail_why.clone(),
                        });
                        remaining -= add_n;
                    }
                }

                if !ok_all {
                    continue;
                }
                let shape = if can_adapt_commander {
                    Shape::Commander
                } else {
                    Shape::Constructed
                };
                if !formats::check(shape, &adapted_main, &cmd_ok).is_empty() {
                    continue;
                }

                let slug_id = rel_precon.replace('/', "-").to_ascii_lowercase();
                let norm_rel = format!("data/decks/imported/adapted/{slug_id}-adapted.txt");
                let adapted_name = format!("{meta_name} (Adapted)");
                let norm_text = format_house_deck_text(
                    &adapted_name,
                    &format!("adapted from data/decks/precon/{rel_precon}.txt ({meta_source})"),
                    "MIT (Copyright © 2018 – Present, Zach Halpern); see NOTICE",
                    &adapted_main,
                    &[],
                    &cmd_ok,
                );
                fs::write(root.join(&norm_rel), norm_text)?;

                let (arch, tags, colors) = classify_deck_strategy(&adapted_main, &cmd_ok);
                let multiset = compute_multiset(&adapted_main, &cmd_ok);
                let deck_hash = compute_deck_hash(&multiset, &cmd_ok);
                let cmd_names: Vec<String> = cmd_ok
                    .iter()
                    .filter_map(|&c| baylee_cards::by_index(c).map(|d| d.name().to_string()))
                    .collect();

                candidates.push(CandidateDeck {
                    id: format!("adapted/{rel_precon}"),
                    name: adapted_name,
                    rel_path: norm_rel,
                    format: if can_adapt_commander {
                        "commander".to_string()
                    } else {
                        "constructed".to_string()
                    },
                    shape_valid: true,
                    archetype: arch,
                    strategy_tags: tags,
                    strategy_confidence: "heuristic_from_dsl",
                    colors: color_string(colors),
                    source_key: "mtgjson_precon_adapted".to_string(),
                    source_ref: format!("data/decks/precon/{rel_precon}.txt ({meta_source})"),
                    source_date: meta_released,
                    license: "MIT (MTGJSON 5.3.0 derived)".to_string(),
                    is_derived: true,
                    original_ref: Some(format!("data/decks/precon/{rel_precon}.txt")),
                    adaptations,
                    main_cards: adapted_main,
                    side_cards: Vec::new(),
                    sideboard_supported: true,
                    commander_names: cmd_names,
                    multiset,
                    contains_held_out_cards: false,
                    held_out_card_count: 0,
                    deck_hash,
                    forced_split: None,
                });
            }
        }
    }

    // -------------------------------------------------------------------------
    // 4. Generate 120 diverse, 100% unmodified (`is_derived: false`) Working decks
    //    spanning Constructed (60), Highlander (100), and Commander (100) across
    //    all 4 archetypes and color combinations, plus dedicated unseen-card test decks!
    // -------------------------------------------------------------------------
    let empty_set = BTreeSet::new();
    let train_pool = deckgen::pool_for_working(&working, &held_out_set);
    let full_pool = deckgen::pool_for_working(&working, &empty_set);

    let shapes = [Shape::Constructed, Shape::Highlander, Shape::Commander];
    let mut curated_usage: BTreeMap<CardIndex, usize> = BTreeMap::new();
    let mut gen_idx = 0_usize;
    let mut seed_cursor = 2026_1022_u64;
    while gen_idx < 105 && seed_cursor < 2026_5000 {
        let shape = shapes[gen_idx % shapes.len()];
        let arch = Archetype::ALL[(gen_idx / shapes.len()) % Archetype::ALL.len()];
        seed_cursor += 1;
        let req = deckgen::DeckBuildRequest {
            shape,
            archetype: arch,
            seed: seed_cursor,
            anchor: None,
            forced_colors: None,
            deficits: None,
            usage_counts: Some(&curated_usage),
        };
        let Ok(g) = deckgen::build_synergistic(&train_pool, &req) else {
            continue;
        };
        if !formats::check(shape, &g.main, &g.leaders).is_empty() {
            continue;
        }
        let fmt_str = deckgen::shape_name(shape);
        let slug = format!("curated-{fmt_str}-{}-{gen_idx:03}", arch.name());
        let norm_rel = format!("data/decks/imported/curated/{slug}.txt");
        let norm_text = format_house_deck_text(
            &g.name,
            &format!(
                "baylee curated {fmt_str} {} seed={seed_cursor}",
                arch.name()
            ),
            "AGPL-3.0-only",
            &g.main,
            &[],
            &g.leaders,
        );
        fs::write(root.join(&norm_rel), norm_text)?;

        let (_, mut tags, colors) = classify_deck_strategy(&g.main, &g.leaders);
        if !tags.contains(&arch.name().to_string()) {
            tags.push(arch.name().to_string());
            tags.sort();
        }
        let multiset = compute_multiset(&g.main, &g.leaders);
        for &cid in multiset.keys() {
            *curated_usage.entry(cid).or_insert(0) += 1;
        }
        let deck_hash = compute_deck_hash(&multiset, &g.leaders);
        let cmd_names: Vec<String> = g
            .leaders
            .iter()
            .filter_map(|&c| baylee_cards::by_index(c).map(|d| d.name().to_string()))
            .collect();

        candidates.push(CandidateDeck {
            id: format!("curated/{slug}"),
            name: g.name,
            rel_path: norm_rel,
            format: fmt_str.to_string(),
            shape_valid: true,
            archetype: arch.name().to_string(),
            strategy_tags: tags,
            strategy_confidence: "generator_spec_and_dsl",
            colors: color_string(colors),
            source_key: "curated_archetype".to_string(),
            source_ref: format!("baylee_train::deckgen seed={seed_cursor}"),
            source_date: "2026-10-22".to_string(),
            license: "AGPL-3.0-only".to_string(),
            is_derived: false,
            original_ref: None,
            adaptations: Vec::new(),
            main_cards: g.main,
            side_cards: Vec::new(),
            sideboard_supported: true,
            commander_names: cmd_names,
            multiset,
            contains_held_out_cards: false,
            held_out_card_count: 0,
            deck_hash,
            forced_split: None,
        });
        gen_idx += 1;
    }

    // Also build 18 dedicated zero-shot unseen-card (`held_out`) test decks across all 3 shapes!
    let mut unseen_idx = 0_usize;
    let mut unseen_seed = 9000_0001_u64;
    while unseen_idx < 18 && unseen_seed < 9000_5000 {
        let shape = shapes[unseen_idx % shapes.len()];
        let arch = Archetype::ALL[(unseen_idx / shapes.len()) % Archetype::ALL.len()];
        unseen_seed += 1;
        let req = deckgen::DeckBuildRequest {
            shape,
            archetype: arch,
            seed: unseen_seed,
            anchor: None,
            forced_colors: None,
            deficits: None,
            usage_counts: Some(&curated_usage),
        };
        let Ok(g) = deckgen::build_synergistic(&full_pool, &req) else {
            continue;
        };
        let multiset = compute_multiset(&g.main, &g.leaders);
        let held_cnt = multiset.keys().filter(|c| held_out_set.contains(c)).count();
        if held_cnt < 2 {
            continue;
        }
        for &cid in multiset.keys() {
            *curated_usage.entry(cid).or_insert(0) += 1;
        }
        let fmt_str = deckgen::shape_name(shape);
        let slug = format!("holdout-cards-{fmt_str}-{}-{unseen_idx:02}", arch.name());
        let norm_rel = format!("data/decks/imported/curated/{slug}.txt");
        let norm_text = format_house_deck_text(
            &g.name,
            &format!("baylee unseen-card holdout {fmt_str} seed={unseen_seed}"),
            "AGPL-3.0-only",
            &g.main,
            &[],
            &g.leaders,
        );
        fs::write(root.join(&norm_rel), norm_text)?;

        let (_, mut tags, colors) = classify_deck_strategy(&g.main, &g.leaders);
        if !tags.contains(&arch.name().to_string()) {
            tags.push(arch.name().to_string());
            tags.sort();
        }
        let deck_hash = compute_deck_hash(&multiset, &g.leaders);
        let cmd_names: Vec<String> = g
            .leaders
            .iter()
            .filter_map(|&c| baylee_cards::by_index(c).map(|d| d.name().to_string()))
            .collect();

        candidates.push(CandidateDeck {
            id: format!("holdout/{slug}"),
            name: g.name,
            rel_path: norm_rel,
            format: fmt_str.to_string(),
            shape_valid: true,
            archetype: arch.name().to_string(),
            strategy_tags: tags,
            strategy_confidence: "generator_spec_and_dsl",
            colors: color_string(colors),
            source_key: "curated_holdout_cards".to_string(),
            source_ref: format!("baylee_train::deckgen holdout seed={unseen_seed}"),
            source_date: "2026-10-22".to_string(),
            license: "AGPL-3.0-only".to_string(),
            is_derived: false,
            original_ref: None,
            adaptations: Vec::new(),
            main_cards: g.main,
            side_cards: Vec::new(),
            sideboard_supported: true,
            commander_names: cmd_names,
            multiset,
            contains_held_out_cards: true,
            held_out_card_count: held_cnt,
            deck_hash,
            forced_split: Some(("test".to_string(), "unseen_cards".to_string())),
        });
        unseen_idx += 1;
    }

    // -------------------------------------------------------------------------
    // 5. Deduplicate via canonical card-multiset Jaccard similarity (`J < max_within_jaccard`)
    // -------------------------------------------------------------------------
    let mut deduped: Vec<CandidateDeck> = Vec::new();
    let mut dedup_hashes: BTreeSet<String> = BTreeSet::new();
    let mut dedup_sim_index = MultisetSimilarityIndex::new();
    let mut dropped_duplicates = 0_usize;
    for cand in candidates {
        let is_dup = dedup_hashes.contains(&cand.deck_hash)
            || dedup_sim_index.max_jaccard(&cand.multiset) >= args.max_within_jaccard;
        if is_dup {
            dropped_duplicates += 1;
            continue;
        }
        dedup_hashes.insert(cand.deck_hash.clone());
        dedup_sim_index.add(&cand.multiset);
        deduped.push(cand);
    }

    // -------------------------------------------------------------------------
    // 6. Group connected components with `J > max_cross_split_jaccard` (`0.85`)
    //    and assign deterministic `train / val / test` splits with ZERO cross-split leakage!
    // -------------------------------------------------------------------------
    let n_base = deduped.len();
    let mut parent: Vec<usize> = (0..n_base).collect();
    for i in 0..n_base {
        for j in (i + 1)..n_base {
            let jacc = multiset_jaccard(&deduped[i].multiset, &deduped[j].multiset);
            if jacc > args.max_cross_split_jaccard {
                let ri = find(&mut parent, i);
                let rj = find(&mut parent, j);
                if ri != rj {
                    parent[rj] = ri;
                }
            }
        }
    }

    let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for i in 0..n_base {
        let r = find(&mut parent, i);
        groups.entry(r).or_default().push(i);
    }

    let mut split_assignment: Vec<(String, String, usize)> =
        vec![(String::new(), String::new(), 0); n_base];
    let mut strat_counters: BTreeMap<(String, String), usize> = BTreeMap::new();

    for (group_id, (_, members)) in groups.iter().enumerate() {
        let mut forced: Option<(String, String)> = None;
        for &m in members {
            if let Some(f) = &deduped[m].forced_split {
                forced = Some(f.clone());
                break;
            }
        }
        let (sp, tier) = if let Some(f) = forced {
            f
        } else {
            let first = &deduped[members[0]];
            let key = (first.format.clone(), first.archetype.clone());
            let idx = *strat_counters.get(&key).unwrap_or(&0);
            strat_counters.insert(key, idx + 1);
            match idx % 10 {
                7 => ("val".to_string(), "val_known_cards".to_string()),
                8 => ("test".to_string(), "unseen_deck_known_cards".to_string()),
                _ => ("train".to_string(), "train_active".to_string()),
            }
        };
        for &m in members {
            if deduped[m].contains_held_out_cards && sp != "test" {
                split_assignment[m] = ("test".to_string(), "unseen_cards".to_string(), group_id);
            } else {
                split_assignment[m] = (sp.clone(), tier.clone(), group_id);
            }
        }
    }

    // -------------------------------------------------------------------------
    // 6b. Coverage-Constrained Synergistic Deck Generation (`k >= min_card_coverage`)
    //     Ensures every working `train_safe` card appears in `>= k` train decks and
    //     every working `held_out` card appears in `>= k` unseen-card test decks!
    // -------------------------------------------------------------------------
    let mut next_group_id = groups.len();
    let mut train_cov_report = None;
    let mut holdout_cov_report = None;

    if args.min_card_coverage > 0 {
        let mut train_ms: Vec<BTreeMap<CardIndex, u32>> = Vec::new();
        let mut non_train_ms: Vec<BTreeMap<CardIndex, u32>> = Vec::new();
        for (idx, d) in deduped.iter().enumerate() {
            if split_assignment[idx].0 == "train" {
                train_ms.push(d.multiset.clone());
            } else {
                non_train_ms.push(d.multiset.clone());
            }
        }

        let train_cfg = CoverageConfig {
            min_coverage: args.min_card_coverage,
            max_jaccard: (args.max_cross_split_jaccard - 0.03).clamp(0.50, 0.84),
            base_seed: 2026_6000,
            max_new_decks: 1200,
        };
        let (train_cov_decks, t_rep) =
            build_coverage_corpus_scoped(&train_pool, None, &train_ms, &non_train_ms, &train_cfg)?;
        println!(
            "[import_decks] Train card coverage (k >= {}): under-covered {} -> {} ({} new synergistic decks)",
            args.min_card_coverage,
            t_rep.under_covered_before,
            t_rep.under_covered_after,
            t_rep.decks_generated
        );
        train_cov_report = Some(t_rep);

        for (cov_idx, g) in train_cov_decks.into_iter().enumerate() {
            let fmt_str = deckgen::shape_name(g.shape);
            let slug = format!(
                "coverage-train-{fmt_str}-{}-{cov_idx:03}",
                g.archetype.name()
            );
            let norm_rel = format!("data/decks/imported/curated/{slug}.txt");
            let norm_text = format_house_deck_text(
                &g.name,
                &format!(
                    "baylee synergistic coverage {fmt_str} {}",
                    g.archetype.name()
                ),
                "AGPL-3.0-only",
                &g.main,
                &[],
                &g.leaders,
            );
            fs::write(root.join(&norm_rel), norm_text)?;

            let (_, mut tags, colors) = classify_deck_strategy(&g.main, &g.leaders);
            if !tags.contains(&g.archetype.name().to_string()) {
                tags.push(g.archetype.name().to_string());
                tags.sort();
            }
            let multiset = compute_multiset(&g.main, &g.leaders);
            let deck_hash = compute_deck_hash(&multiset, &g.leaders);
            let cmd_names: Vec<String> = g
                .leaders
                .iter()
                .filter_map(|&c| baylee_cards::by_index(c).map(|d| d.name().to_string()))
                .collect();

            let gid = next_group_id;
            next_group_id += 1;
            deduped.push(CandidateDeck {
                id: format!("coverage/{slug}"),
                name: g.name,
                rel_path: norm_rel,
                format: fmt_str.to_string(),
                shape_valid: true,
                archetype: g.archetype.name().to_string(),
                strategy_tags: tags,
                strategy_confidence: "generator_spec_and_dsl",
                colors: color_string(colors),
                source_key: "curated_coverage".to_string(),
                source_ref: format!("baylee_train::deckgen::coverage train #{cov_idx}"),
                source_date: "2026-10-25".to_string(),
                license: "AGPL-3.0-only".to_string(),
                is_derived: false,
                original_ref: None,
                adaptations: Vec::new(),
                main_cards: g.main,
                side_cards: Vec::new(),
                sideboard_supported: true,
                commander_names: cmd_names,
                multiset,
                contains_held_out_cards: false,
                held_out_card_count: 0,
                deck_hash,
                forced_split: Some(("train".to_string(), "train_active".to_string())),
            });
            split_assignment.push(("train".to_string(), "train_active".to_string(), gid));
        }

        // Now ensure every held-out 5% zero-shot test card appears in >= k test decks
        let mut test_ms: Vec<BTreeMap<CardIndex, u32>> = Vec::new();
        let mut non_test_ms: Vec<BTreeMap<CardIndex, u32>> = Vec::new();
        for (idx, d) in deduped.iter().enumerate() {
            if split_assignment[idx].0 == "test" {
                test_ms.push(d.multiset.clone());
            } else {
                non_test_ms.push(d.multiset.clone());
            }
        }
        let holdout_cfg = CoverageConfig {
            min_coverage: args.min_card_coverage,
            max_jaccard: (args.max_cross_split_jaccard - 0.03).clamp(0.50, 0.84),
            base_seed: 9001_0000,
            max_new_decks: 400,
        };
        let (holdout_cov_decks, h_rep) = build_coverage_corpus_scoped(
            &full_pool,
            Some(&held_out_set),
            &test_ms,
            &non_test_ms,
            &holdout_cfg,
        )?;
        println!(
            "[import_decks] Holdout test card coverage (k >= {}): under-covered {} -> {} ({} new synergistic decks)",
            args.min_card_coverage,
            h_rep.under_covered_before,
            h_rep.under_covered_after,
            h_rep.decks_generated
        );
        holdout_cov_report = Some(h_rep);

        for (cov_idx, g) in holdout_cov_decks.into_iter().enumerate() {
            let fmt_str = deckgen::shape_name(g.shape);
            let slug = format!(
                "coverage-holdout-{fmt_str}-{}-{cov_idx:03}",
                g.archetype.name()
            );
            let norm_rel = format!("data/decks/imported/curated/{slug}.txt");
            let norm_text = format_house_deck_text(
                &g.name,
                &format!(
                    "baylee synergistic holdout coverage {fmt_str} {}",
                    g.archetype.name()
                ),
                "AGPL-3.0-only",
                &g.main,
                &[],
                &g.leaders,
            );
            fs::write(root.join(&norm_rel), norm_text)?;

            let (_, mut tags, colors) = classify_deck_strategy(&g.main, &g.leaders);
            if !tags.contains(&g.archetype.name().to_string()) {
                tags.push(g.archetype.name().to_string());
                tags.sort();
            }
            let multiset = compute_multiset(&g.main, &g.leaders);
            let held_cnt = multiset.keys().filter(|c| held_out_set.contains(c)).count();
            let deck_hash = compute_deck_hash(&multiset, &g.leaders);
            let cmd_names: Vec<String> = g
                .leaders
                .iter()
                .filter_map(|&c| baylee_cards::by_index(c).map(|d| d.name().to_string()))
                .collect();

            let gid = next_group_id;
            next_group_id += 1;
            deduped.push(CandidateDeck {
                id: format!("holdout/{slug}"),
                name: g.name,
                rel_path: norm_rel,
                format: fmt_str.to_string(),
                shape_valid: true,
                archetype: g.archetype.name().to_string(),
                strategy_tags: tags,
                strategy_confidence: "generator_spec_and_dsl",
                colors: color_string(colors),
                source_key: "curated_holdout_cards".to_string(),
                source_ref: format!("baylee_train::deckgen::coverage holdout #{cov_idx}"),
                source_date: "2026-10-25".to_string(),
                license: "AGPL-3.0-only".to_string(),
                is_derived: false,
                original_ref: None,
                adaptations: Vec::new(),
                main_cards: g.main,
                side_cards: Vec::new(),
                sideboard_supported: true,
                commander_names: cmd_names,
                multiset,
                contains_held_out_cards: held_cnt > 0,
                held_out_card_count: held_cnt,
                deck_hash,
                forced_split: Some(("test".to_string(), "unseen_cards".to_string())),
            });
            split_assignment.push(("test".to_string(), "unseen_cards".to_string(), gid));
        }
    }

    // Build per-split inverted indices to verify `J(A, B) <= max_cross_split_jaccard` across splits
    let n = deduped.len();
    let mut train_idx = MultisetSimilarityIndex::new();
    let mut val_idx = MultisetSimilarityIndex::new();
    let mut test_idx = MultisetSimilarityIndex::new();
    for i in 0..n {
        match split_assignment[i].0.as_str() {
            "train" => train_idx.add(&deduped[i].multiset),
            "val" => val_idx.add(&deduped[i].multiset),
            _ => test_idx.add(&deduped[i].multiset),
        }
    }

    let mut entries: Vec<ManifestDeckEntry> = Vec::with_capacity(n);
    let mut max_observed_cross_jaccard = 0.0_f64;
    for i in 0..n {
        let (ref sp_i, ref tier_i, gid_i) = split_assignment[i];
        let ms_i = &deduped[i].multiset;
        let max_cross = match sp_i.as_str() {
            "train" => val_idx.max_jaccard(ms_i).max(test_idx.max_jaccard(ms_i)),
            "val" => train_idx.max_jaccard(ms_i).max(test_idx.max_jaccard(ms_i)),
            _ => train_idx.max_jaccard(ms_i).max(val_idx.max_jaccard(ms_i)),
        };
        assert!(
            max_cross <= args.max_cross_split_jaccard + 1e-9,
            "Cross-split Jaccard violation: deck {} ({}) has J={:.4} > {:.2}",
            deduped[i].id,
            sp_i,
            max_cross,
            args.max_cross_split_jaccard
        );
        max_observed_cross_jaccard = max_observed_cross_jaccard.max(max_cross);

        let d = &deduped[i];
        entries.push(ManifestDeckEntry {
            id: d.id.clone(),
            name: d.name.clone(),
            path: d.rel_path.clone(),
            format: d.format.clone(),
            shape_valid: d.shape_valid,
            archetype: d.archetype.clone(),
            strategy_tags: d.strategy_tags.clone(),
            strategy_confidence: d.strategy_confidence,
            colors: d.colors.clone(),
            source_key: d.source_key.clone(),
            source_ref: d.source_ref.clone(),
            source_date: d.source_date.clone(),
            license: d.license.clone(),
            is_derived: d.is_derived,
            original_ref: d.original_ref.clone(),
            adaptations: d.adaptations.clone(),
            main_count: d.main_cards.len(),
            sideboard_count: d.side_cards.len(),
            sideboard_supported: d.sideboard_supported,
            commanders: d.commander_names.clone(),
            distinct_cards: d.multiset.len(),
            contains_held_out_cards: d.contains_held_out_cards,
            held_out_card_count: d.held_out_card_count,
            deck_hash: d.deck_hash.clone(),
            similarity_group_id: gid_i,
            max_cross_split_jaccard: (max_cross * 10000.0).round() / 10000.0,
            holdout_tier: tier_i.clone(),
            split: sp_i.clone(),
        });
    }

    // Summary counters
    let mut by_split: BTreeMap<String, usize> = BTreeMap::new();
    let mut by_format: BTreeMap<String, usize> = BTreeMap::new();
    let mut by_source: BTreeMap<String, usize> = BTreeMap::new();
    let mut by_archetype: BTreeMap<String, usize> = BTreeMap::new();
    let mut by_tag: BTreeMap<String, usize> = BTreeMap::new();
    let mut by_tier: BTreeMap<String, usize> = BTreeMap::new();
    let mut original_count = 0_usize;
    let mut derived_count = 0_usize;

    for e in &entries {
        *by_split.entry(e.split.clone()).or_insert(0) += 1;
        *by_format.entry(e.format.clone()).or_insert(0) += 1;
        *by_source.entry(e.source_key.clone()).or_insert(0) += 1;
        *by_archetype.entry(e.archetype.clone()).or_insert(0) += 1;
        *by_tier.entry(e.holdout_tier.clone()).or_insert(0) += 1;
        for t in &e.strategy_tags {
            *by_tag.entry(t.clone()).or_insert(0) += 1;
        }
        if e.is_derived {
            derived_count += 1;
        } else {
            original_count += 1;
        }
    }

    let manifest_json = serde_json::json!({
        "importer_version": IMPORTER_VERSION,
        "generated_at": "2026-10-25",
        "working_pool_cards": working.cards.len(),
        "working_pool_hash": working.hash(),
        "held_out_every": 20,
        "held_out_cards_count": held_out_set.len(),
        "min_card_coverage_target": args.min_card_coverage,
        "train_card_coverage_after_min": train_cov_report.as_ref().map(|r| r.min_coverage_after),
        "train_under_covered_after": train_cov_report.as_ref().map(|r| r.under_covered_after),
        "holdout_card_coverage_after_min": holdout_cov_report.as_ref().map(|r| r.min_coverage_after),
        "holdout_under_covered_after": holdout_cov_report.as_ref().map(|r| r.under_covered_after),
        "total_accepted_decks": entries.len(),
        "original_unmodified_decks": original_count,
        "derived_adapted_decks": derived_count,
        "dropped_near_duplicates": dropped_duplicates,
        "rejected_precon_decks": rejected.len(),
        "max_within_jaccard_threshold": args.max_within_jaccard,
        "max_cross_split_jaccard_threshold": args.max_cross_split_jaccard,
        "max_observed_cross_split_jaccard": (max_observed_cross_jaccard * 10000.0).round() / 10000.0,
        "counts_by_split": by_split,
        "counts_by_format": by_format,
        "counts_by_source": by_source,
        "counts_by_archetype": by_archetype,
        "counts_by_strategy_tag": by_tag,
        "counts_by_holdout_tier": by_tier,
        "decks": entries,
    });

    fs::write(
        root.join(&args.manifest_out),
        serde_json::to_vec_pretty(&manifest_json)?,
    )?;
    fs::write(
        &args.rejected_out,
        serde_json::to_vec_pretty(&serde_json::json!({
            "importer_version": IMPORTER_VERSION,
            "working_pool_hash": working.hash(),
            "rejected_count": rejected.len(),
            "rejected": rejected,
        }))?,
    )?;

    // Write human-readable report (`docs/history/deck-corpus-v4-2026-10-22.md`)
    let mut rep = String::new();
    let _ = writeln!(
        rep,
        "# Playernet v4 Deck Corpus & Split Report (2026-10-22)\n"
    );
    let _ = writeln!(rep, "- **Importer Version:** `{IMPORTER_VERSION}`");
    let _ = writeln!(
        rep,
        "- **Working Card Pool:** `{}` cards (`hash = {}`), with `{}` cards (`5%`) held out in `deckgen::held_out(&working, 20)`",
        working.cards.len(),
        working.hash(),
        held_out_set.len()
    );
    let _ = writeln!(
        rep,
        "- **Accepted Active Decks:** `{}` (`{}` unmodified original `is_derived = false`, `{}` rule-validated adapted precon derivations `is_derived = true`)",
        entries.len(),
        original_count,
        derived_count
    );
    let _ = writeln!(
        rep,
        "- **Near-Duplicates Dropped (`J >= {:.2}`):** `{}`",
        args.max_within_jaccard, dropped_duplicates
    );
    let _ = writeln!(
        rep,
        "- **Max Observed Cross-Split Jaccard (`train` vs `val`/`test`):** `{:.4}` (strictly `<= {:.2}`)",
        max_observed_cross_jaccard, args.max_cross_split_jaccard
    );
    let _ = writeln!(
        rep,
        "- **Rejected Precon Decks Logged Outside Pool:** `{}` (`{}`)\n",
        rejected.len(),
        args.rejected_out.display()
    );

    let _ = writeln!(rep, "## Counts by Split\n\n| Split | Decks |\n|---|---:|");
    for (k, v) in &by_split {
        let _ = writeln!(rep, "| `{k}` | {v} |");
    }
    let _ = writeln!(
        rep,
        "\n## Counts by Holdout Tier\n\n| Holdout Tier | Decks |\n|---|---:|"
    );
    for (k, v) in &by_tier {
        let _ = writeln!(rep, "| `{k}` | {v} |");
    }
    let _ = writeln!(
        rep,
        "\n## Counts by Format\n\n| Format | Decks |\n|---|---:|"
    );
    for (k, v) in &by_format {
        let _ = writeln!(rep, "| `{k}` | {v} |");
    }
    let _ = writeln!(
        rep,
        "\n## Counts by Source\n\n| Source | Decks |\n|---|---:|"
    );
    for (k, v) in &by_source {
        let _ = writeln!(rep, "| `{k}` | {v} |");
    }
    let _ = writeln!(
        rep,
        "\n## Counts by Primary Archetype\n\n| Archetype | Decks |\n|---|---:|"
    );
    for (k, v) in &by_archetype {
        let _ = writeln!(rep, "| `{k}` | {v} |");
    }
    let _ = writeln!(
        rep,
        "\n## Counts by Strategy Tag\n\n| Strategy Tag | Decks |\n|---|---:|"
    );
    for (k, v) in &by_tag {
        let _ = writeln!(rep, "| `{k}` | {v} |");
    }
    fs::write(root.join(&args.report_out), rep)?;

    println!(
        "[import_decks] Wrote {} decks to {} ({} original, {} derived; max_cross_split_jaccard={:.4})",
        entries.len(),
        args.manifest_out.display(),
        original_count,
        derived_count,
        max_observed_cross_jaccard
    );
    println!("  by split: {by_split:?}");
    println!("  by format: {by_format:?}");
    println!("  by source: {by_source:?}");
    println!("  by holdout_tier: {by_tier:?}");
    let _ = Rng::new(1);
    Ok(())
}
