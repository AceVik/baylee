//! Extends a deck manifest with the decks that became complete since it was
//! written, and changes nothing else: every existing entry keeps its place,
//! its split and its bytes, and a new deck's split is a function of its own
//! cards alone, so two runs a week apart put the same deck in the same split.
//!
//! New decks come from the precon lists (`data/decks/precon`) and from
//! near-complete MTGO lists (`mtgo_near_complete_v1`). A deck is taken when
//! every card of its main deck and command zone works (`working`), under the
//! rules the two importers already apply:
//!
//! - a deck holding a held-out card (`deckgen::held_out(&working, 20)`) is a
//!   test deck (`unseen_cards`);
//! - otherwise its split is `import_mtgo4`'s: the first eight hex digits of
//!   SHA-256 over its `(index, count)` pairs, modulo ten — 0 test, 1 val,
//!   anything else train;
//! - a deck at or above `--similarity` multiset Jaccard to a deck already in
//!   any manifest given, or to one taken before it, is dropped. The one
//!   exception is a precon whose adapted copy (`adapted/<key>`) is that deck
//!   and sits in `train`: the original joins it there, as the precon importer
//!   groups near decks into one split;
//! - nothing comes near a protected evaluation pool (`--protect`): a deck at
//!   or above `--similarity` to one of its decks, a precon whose source list
//!   is a protected deck's `original_ref` or whose `adapted/<key>` copy is
//!   protected, and a precon whose adapted copy sits outside `train` are all
//!   dropped, whatever their similarity.
//!
//! Without `--write` it prints what it would add and writes nothing. With
//! it, it writes a new directory (which must not exist): the deck files and
//! a copy of the base manifest with the new entries appended, then reads the
//! copy back and checks every old entry against the original.

// Looks cards up, so this program holds the pool (`baylee-cards-pool`).
use baylee_cards_pool as _;

#[path = "import_decks/document.rs"]
mod document;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, bail, ensure};
use baylee_cards::deckgen::strategy::{classify_deck_strategy, color_string};
use baylee_cards::decks::{by_name, leader_of, may_lead_together};
use baylee_cards::formats::{self, Shape};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_train::deckgen;
use baylee_train::housedeck::HouseDeck;
use baylee_train::working::{Working, repo_root};
use clap::Parser;
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};

#[derive(Parser)]
#[command(about = "Appends newly complete decks to a copy of a deck manifest")]
struct Args {
    /// The manifest extended; it is never written.
    #[arg(long)]
    manifest: PathBuf,
    /// Further manifests whose decks exist already (keys and similarity).
    #[arg(long)]
    existing: Vec<PathBuf>,
    /// Evaluation pools no new deck may come near (repeatable): a new deck
    /// at or above `--similarity` to one of their decks, or made from the
    /// same source list (`original_ref`, `adapted/<key>`), is dropped.
    #[arg(long)]
    protect: Vec<PathBuf>,
    /// Near-complete MTGO lists to admit once complete (repeatable).
    #[arg(long)]
    near_complete: Vec<PathBuf>,
    /// Also admit the precon lists under `data/decks/precon`.
    #[arg(long)]
    precons: bool,
    /// Drop a new deck at or above this multiset Jaccard to a kept deck.
    #[arg(long, default_value_t = 0.85)]
    similarity: f64,
    /// The new directory: `manifest.json` and `decks/`.
    #[arg(long)]
    out: PathBuf,
    /// Write it. Without this flag nothing is written.
    #[arg(long)]
    write: bool,
}

/// A candidate deck, complete on this build.
struct Candidate {
    id: String,
    name: String,
    source_key: &'static str,
    source_ref: String,
    source_date: String,
    license: String,
    /// The importer's own `format` word.
    format: &'static str,
    source_format: Option<String>,
    main: Vec<CardIndex>,
    commanders: Vec<CardIndex>,
}

impl Candidate {
    fn multiset(&self) -> BTreeMap<u32, u64> {
        let mut m = BTreeMap::new();
        for c in self.main.iter().chain(&self.commanders) {
            *m.entry(c.get()).or_default() += 1;
        }
        m
    }
    /// `import_mtgo4`'s identity: SHA-256 over the JSON `(index, count)` pairs.
    fn identity(&self) -> Result<String> {
        let pairs: Vec<(u64, u64)> = self
            .multiset()
            .into_iter()
            .map(|(c, k)| (u64::from(c), k))
            .collect();
        Ok(Sha256::digest(serde_json::to_vec(&pairs)?)
            .iter()
            .fold(String::new(), |mut s, b| {
                let _ = write!(s, "{b:02x}");
                s
            }))
    }
}

/// Multiset Jaccard over an inverted index, as `import_mtgo4` measures it.
#[derive(Default)]
struct Similar {
    postings: BTreeMap<u32, Vec<(usize, u64)>>,
    sizes: Vec<u64>,
    ids: Vec<String>,
    splits: Vec<String>,
}

impl Similar {
    fn add(&mut self, id: &str, split: &str, cards: &BTreeMap<u32, u64>) {
        let n = self.sizes.len();
        for (c, k) in cards {
            self.postings.entry(*c).or_default().push((n, *k));
        }
        self.sizes.push(cards.values().sum());
        self.ids.push(id.to_owned());
        self.splits.push(split.to_owned());
    }
    /// The nearest kept deck and its share.
    fn nearest(&self, cards: &BTreeMap<u32, u64>) -> Option<(f64, &str)> {
        self.shares(cards)
            .max_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(s, id, _)| (s, id))
    }
    /// The largest share with a kept deck in another split than `split`.
    fn nearest_across(&self, cards: &BTreeMap<u32, u64>, split: &str) -> f64 {
        self.shares(cards)
            .filter(|(_, _, s)| *s != split)
            .map(|(share, _, _)| share)
            .fold(0.0, f64::max)
    }
    /// Every kept deck sharing a card with `cards`: its share, id and split.
    #[allow(clippy::cast_precision_loss, reason = "a few hundred cards")]
    fn shares(&self, cards: &BTreeMap<u32, u64>) -> impl Iterator<Item = (f64, &str, &str)> {
        let mut inter: BTreeMap<usize, u64> = BTreeMap::new();
        for (c, k) in cards {
            for (id, other) in self.postings.get(c).into_iter().flatten() {
                *inter.entry(*id).or_default() += (*k).min(*other);
            }
        }
        let size: u64 = cards.values().sum();
        inter.into_iter().map(move |(id, n)| {
            (
                n as f64 / (size + self.sizes[id] - n) as f64,
                self.ids[id].as_str(),
                self.splits[id].as_str(),
            )
        })
    }
}

/// The decks of the evaluation pools: ids, source lists and lists.
#[derive(Default)]
struct Protected {
    ids: BTreeSet<String>,
    refs: BTreeSet<String>,
    similar: Similar,
    decks: usize,
}

impl Protected {
    fn read(root: &Path, manifests: &[PathBuf]) -> Result<Self> {
        let mut protected = Self::default();
        for path in manifests {
            let manifest = read_json(path)?;
            for e in manifest["decks"]
                .as_array()
                .context("a manifest without decks")?
            {
                let id = e["id"].as_str().context("a deck without an id")?.to_owned();
                if let Some(original) = e["original_ref"].as_str() {
                    protected.refs.insert(original.to_owned());
                }
                let list = resolve_path(root, e["path"].as_str().unwrap_or_default());
                let text = fs::read_to_string(&list)
                    .with_context(|| format!("protected deck {id}: {}", list.display()))?;
                let deck = HouseDeck::parse_any(&id, &text)?;
                let mut m = BTreeMap::new();
                for c in deck.deck.main.iter().chain(&deck.deck.commanders) {
                    *m.entry(c.index.get()).or_default() += 1;
                }
                protected.similar.add(&id, "protected", &m);
                protected.ids.insert(id);
                protected.decks += 1;
            }
        }
        Ok(protected)
    }
}

fn read_json(path: &Path) -> Result<Value> {
    serde_json::from_slice(&fs::read(path).with_context(|| path.display().to_string())?)
        .with_context(|| path.display().to_string())
}

fn resolve_path(root: &Path, path: &str) -> PathBuf {
    if Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        root.join(path)
    }
}

/// A working card named `name`, if there is one.
fn working_card(working: &Working, name: &str) -> Option<CardIndex> {
    by_name(name).filter(|c| working.check(*c).is_ok())
}

/// The precon lists that are complete on this build.
fn precons(root: &Path, working: &Working) -> Result<Vec<Candidate>> {
    let precon_root = root.join("data/decks/precon");
    let mut files = Vec::new();
    for set in fs::read_dir(&precon_root)? {
        let set = set?.path();
        if set.is_dir() {
            for f in fs::read_dir(&set)? {
                let f = f?.path();
                if f.extension().is_some_and(|e| e == "txt") {
                    files.push(f);
                }
            }
        }
    }
    files.sort();
    let mut out = Vec::new();
    'deck: for path in files {
        let key = path
            .strip_prefix(&precon_root)?
            .with_extension("")
            .to_string_lossy()
            .replace('\\', "/");
        let text = fs::read_to_string(&path)?;
        let meta = |field: &str| {
            text.lines()
                .find_map(|l| l.strip_prefix(&format!("# {field}: ")).map(str::trim))
                .map(str::to_owned)
        };
        let Ok(baylee_deckio::Import::Read { read, .. }) = baylee_deckio::import(&text) else {
            continue;
        };
        let (mut main, mut commanders) = (Vec::new(), Vec::new());
        for card in &read.document.cards {
            let zone = card.zone;
            if !matches!(
                zone,
                baylee_deckio::Zone::Main | baylee_deckio::Zone::Commander
            ) {
                continue;
            }
            let Some(c) = working_card(working, &card.name) else {
                continue 'deck;
            };
            let to = if zone == baylee_deckio::Zone::Commander {
                &mut commanders
            } else {
                &mut main
            };
            to.extend(std::iter::repeat_n(c, card.count as usize));
        }
        let leaders: Option<Vec<_>> = commanders.iter().map(|c| leader_of(*c)).collect();
        let Some(leaders) = leaders else { continue };
        let valid_leaders = match leaders.as_slice() {
            [] => true,
            [one] => one.eligible_alone,
            [a, b] => may_lead_together(a, b),
            _ => false,
        };
        if !valid_leaders || main.is_empty() {
            continue;
        }
        let format = if !commanders.is_empty() {
            "commander"
        } else if main.len() >= 60 {
            "constructed"
        } else {
            "starter_limited"
        };
        let source = meta("source").unwrap_or_default();
        out.push(Candidate {
            id: format!("precon/{key}"),
            name: meta("name").unwrap_or_else(|| key.clone()),
            source_key: "mtgjson_precon",
            source_ref: format!("data/decks/precon/{key}.txt ({source})"),
            source_date: meta("released").unwrap_or_default(),
            license: "MIT (MTGJSON 5.3.0)".to_owned(),
            format,
            source_format: None,
            main,
            commanders,
        });
    }
    Ok(out)
}

/// The near-complete MTGO lists of `path` that are complete on this build.
fn mtgo(path: &Path, working: &Working) -> Result<Vec<Candidate>> {
    let lists = read_json(path)?;
    ensure!(
        lists["contract"] == "mtgo_near_complete_v1",
        "{} is not an mtgo_near_complete_v1 file",
        path.display()
    );
    let mut out = Vec::new();
    'deck: for d in lists["decks"].as_array().context("decks")? {
        let mut main = Vec::new();
        for row in d["main"].as_array().into_iter().flatten() {
            let (Some(name), Some(count)) = (row[0].as_str(), row[1].as_u64()) else {
                continue 'deck;
            };
            let Some(c) = working_card(working, name) else {
                continue 'deck;
            };
            main.extend(std::iter::repeat_n(c, usize::try_from(count)?));
        }
        if !(60..=100).contains(&main.len()) || main.iter().all(|c| formats::is_basic_land(*c)) {
            continue;
        }
        let format = d["source_format"].as_str().unwrap_or("?").to_owned();
        let mut candidate = Candidate {
            id: String::new(),
            name: String::new(),
            source_key: "mtgo_decklistcache",
            source_ref: format!(
                "{}, deck index {}",
                d["source_ref"].as_str().unwrap_or_default(),
                d["source_deck_index"]
            ),
            source_date: d["source_date"].as_str().unwrap_or_default().to_owned(),
            license: d["license"].as_str().unwrap_or("GPL-3.0").to_owned(),
            format: "constructed",
            source_format: Some(format.clone()),
            main,
            commanders: Vec::new(),
        };
        let identity = candidate.identity()?;
        candidate.id = format!("mtgo/{format}/{}", &identity[..20]);
        candidate.name = format!("MTGO {format} {}", &identity[..12]);
        out.push(candidate);
    }
    Ok(out)
}

/// Every count the base manifest keeps, recomputed over `decks`.
fn recount(manifest: &mut Value) {
    let mut counts: BTreeMap<&str, BTreeMap<String, u64>> = BTreeMap::new();
    let decks = manifest["decks"].as_array().cloned().unwrap_or_default();
    for deck in &decks {
        for (field, key) in [
            ("counts_by_split", "split"),
            ("counts_by_format", "format"),
            ("counts_by_source", "source_key"),
            ("counts_by_archetype", "archetype"),
            ("counts_by_holdout_tier", "holdout_tier"),
        ] {
            *counts
                .entry(field)
                .or_default()
                .entry(deck[key].as_str().unwrap_or("?").to_owned())
                .or_default() += 1;
        }
        for tag in deck["strategy_tags"].as_array().into_iter().flatten() {
            *counts
                .entry("counts_by_strategy_tag")
                .or_default()
                .entry(tag.as_str().unwrap_or("?").to_owned())
                .or_default() += 1;
        }
    }
    for (field, map) in counts {
        manifest[field] = json!(map);
    }
    manifest["total_accepted_decks"] = json!(decks.len());
}

#[allow(clippy::too_many_lines, reason = "one pass, in the order it runs")]
fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.similarity.is_finite() && (0.0..=1.0).contains(&args.similarity),
        "similarity must be in [0, 1]"
    );
    if args.write {
        ensure!(!args.out.exists(), "{} exists", args.out.display());
    }
    let root = repo_root();
    let working = Working::scan(&root)?;
    let held = deckgen::held_out(&working, 20);

    // What exists: keys, splits and lists, over every manifest given.
    let base = read_json(&args.manifest)?;
    let mut known: BTreeMap<String, String> = BTreeMap::new();
    let mut kept = Similar::default();
    let mut unreadable = Vec::new();
    let protected = Protected::read(&root, &args.protect)?;
    let mut manifests = vec![base.clone()];
    for p in &args.existing {
        manifests.push(read_json(p)?);
    }
    for m in &manifests {
        for e in m["decks"].as_array().context("a manifest without decks")? {
            let id = e["id"].as_str().context("a deck without an id")?.to_owned();
            known.insert(id.clone(), e["split"].as_str().unwrap_or("?").to_owned());
            let path = resolve_path(&root, e["path"].as_str().unwrap_or_default());
            match fs::read_to_string(&path)
                .map_err(anyhow::Error::from)
                .and_then(|t| HouseDeck::parse_any(&id, &t))
            {
                Ok(deck) => {
                    let mut m = BTreeMap::new();
                    for c in deck.deck.main.iter().chain(&deck.deck.commanders) {
                        *m.entry(c.index.get()).or_default() += 1;
                    }
                    kept.add(&id, e["split"].as_str().unwrap_or("?"), &m);
                }
                Err(_) => unreadable.push(id),
            }
        }
    }

    let mut candidates = Vec::new();
    if args.precons {
        candidates.extend(precons(&root, &working)?);
    }
    for p in &args.near_complete {
        candidates.extend(mtgo(p, &working)?);
    }

    let mut added: Vec<(Candidate, Value)> = Vec::new();
    let mut skipped: BTreeMap<String, u64> = BTreeMap::new();
    let mut seen = BTreeSet::new();
    let mut max_cross = 0.0_f64;
    for c in candidates {
        if known.contains_key(&c.id) || !seen.insert(c.id.clone()) {
            *skipped.entry("already_present".into()).or_default() += 1;
            continue;
        }
        let cards = c.multiset();
        // The original of a deck kept out of training stays out with it: a
        // protected deck's source, or a precon whose adapted copy is outside
        // train, is never added, however far the two lists have drifted.
        if let Some(key) = c.id.strip_prefix("precon/") {
            let adapted = format!("adapted/{key}");
            if protected
                .refs
                .contains(&format!("data/decks/precon/{key}.txt"))
                || protected.ids.contains(&adapted)
            {
                *skipped.entry("original_of_protected".into()).or_default() += 1;
                continue;
            }
            if let Some(split) = known.get(&adapted).filter(|s| *s != "train") {
                *skipped.entry(format!("original_of_{split}")).or_default() += 1;
                continue;
            }
        }
        if protected
            .similar
            .nearest(&cards)
            .is_some_and(|(share, _)| share >= args.similarity)
        {
            *skipped.entry("near_protected".into()).or_default() += 1;
            continue;
        }
        let held_count = cards
            .keys()
            .filter(|k| held.contains(&CardIndex::new(**k)))
            .count();
        let identity = c.identity()?;
        let (mut split, mut tier) = if held_count > 0 {
            ("test", "unseen_cards")
        } else {
            match u32::from_str_radix(&identity[..8], 16)? % 10 {
                0 => ("test", "unseen_deck_known_cards"),
                1 => ("val", "val_known_cards"),
                _ => ("train", "train_active"),
            }
        };
        let nearest = kept.nearest(&cards);
        if let Some((share, near)) = nearest
            && share >= args.similarity
        {
            let own_adaptation =
                c.id.strip_prefix("precon/")
                    .is_some_and(|k| near == format!("adapted/{k}"));
            let near_split = known.get(near).map_or("new", String::as_str);
            if own_adaptation && near_split == "train" && held_count == 0 {
                (split, tier) = ("train", "train_active");
            } else {
                *skipped
                    .entry(format!("near_duplicate_of_{near_split}"))
                    .or_default() += 1;
                continue;
            }
        }
        let cross = kept.nearest_across(&cards, split);
        max_cross = max_cross.max(cross);
        let (archetype, tags, colors) = classify_deck_strategy(&c.main, &c.commanders);
        let file = format!("{}.txt", c.id.replace('/', "-"));
        let path = args.out.join("decks").join(&file);
        let mut entry = json!({
            "archetype": archetype,
            "colors": color_string(colors),
            "commanders": c.commanders.iter().filter_map(|k| baylee_cards::by_index(*k).map(baylee_cards_dsl::CardDef::name)).collect::<Vec<_>>(),
            "contains_held_out_cards": held_count > 0,
            "deck_hash": identity,
            "distinct_cards": cards.len(),
            "format": c.format,
            "held_out_card_count": held_count,
            "holdout_tier": tier,
            "id": c.id,
            "is_derived": false,
            "license": c.license,
            "main_count": c.main.len(),
            "max_cross_split_jaccard": (cross * 10_000.0).round() / 10_000.0,
            "name": c.name,
            "path": path.to_string_lossy(),
            "shape_valid": formats::check(
                if c.commanders.is_empty() { Shape::Constructed } else { Shape::Commander },
                &c.main,
                &c.commanders,
            )
            .is_empty(),
            "sideboard_count": 0,
            "sideboard_supported": false,
            "source_date": c.source_date,
            "source_key": c.source_key,
            "source_ref": c.source_ref,
            "split": split,
            "strategy_confidence": "heuristic_from_dsl",
            "strategy_tags": tags,
            "working_pool_hash": working.hash(),
        });
        if let Some(f) = &c.source_format {
            entry["source_format"] = json!(f);
        }
        kept.add(&c.id, split, &cards);
        added.push((c, entry));
    }

    // The proposal.
    let mut by: BTreeMap<String, u64> = BTreeMap::new();
    for (c, e) in &added {
        let fmt = c.source_format.as_deref().unwrap_or(c.format);
        *by.entry(format!(
            "{} {} {fmt}",
            e["split"].as_str().unwrap_or("?"),
            c.source_key
        ))
        .or_default() += 1;
    }
    let summary = json!({
        "base": args.manifest,
        "base_decks": base["decks"].as_array().map_or(0, Vec::len),
        "base_working_pool_hash": base["working_pool_hash"],
        "working_pool_hash": working.hash(),
        "working_cards": working.cards.len(),
        "existing_unreadable": unreadable,
        "protected": args.protect,
        "protected_decks": protected.decks,
        "added": added.len(),
        "added_by_split_source_format": by,
        "skipped": skipped,
        "max_cross_split_jaccard_new_vs_kept": (max_cross * 10_000.0).round() / 10_000.0,
        "added_ids": added.iter().map(|(_, e)| format!("{} {}", e["split"].as_str().unwrap_or("?"), e["id"].as_str().unwrap_or("?"))).collect::<Vec<_>>(),
    });
    println!("{}", serde_json::to_string_pretty(&summary)?);
    if !args.write {
        eprintln!(
            "dry run: nothing written (pass --write to write {})",
            args.out.display()
        );
        return Ok(());
    }

    // The copy: the base, untouched, with the new entries after it.
    fs::create_dir_all(args.out.join("decks"))?;
    let mut copy = base.clone();
    let decks = copy["decks"].as_array_mut().context("decks")?;
    for (c, e) in &added {
        let text = document::format_house_deck_text(
            &c.name,
            &format!("source: {}", c.source_ref),
            &c.license,
            &c.main,
            &[],
            &c.commanders,
        );
        HouseDeck::parse_any(&c.id, &text)
            .with_context(|| format!("{} does not read back", c.id))?;
        fs::write(e["path"].as_str().context("path")?, text)?;
        decks.push(e.clone());
    }
    copy["working_pool_hash"] = json!(working.hash());
    copy["working_pool_cards"] = json!(working.cards.len());
    copy["extended_from"] = json!({
        "manifest": args.manifest,
        "decks": base["decks"].as_array().map_or(0, Vec::len),
        "working_pool_hash": base["working_pool_hash"],
        "protected": args.protect,
    });
    recount(&mut copy);
    let out = args.out.join("manifest.json");
    fs::write(&out, serde_json::to_vec_pretty(&copy)?)?;

    // Read it back: every old entry exactly as it was, in its place.
    let back = read_json(&out)?;
    let (old, new) = (
        base["decks"].as_array().context("decks")?,
        back["decks"].as_array().context("decks")?,
    );
    if new.len() != old.len() + added.len() || old.iter().zip(new).any(|(a, b)| a != b) {
        bail!("{} does not keep the base entries unchanged", out.display());
    }
    eprintln!(
        "wrote {} ({} decks, {} new)",
        out.display(),
        new.len(),
        added.len()
    );
    Ok(())
}
