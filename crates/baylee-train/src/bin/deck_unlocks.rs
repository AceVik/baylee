//! Ranks the cards that are not working yet by the real decks they would
//! bring into the TRAIN split: a greedy set cover over the precon lists
//! (`data/decks/precon`) and the near-complete MTGO lists, against this
//! checkout's working pool. Analysis only; it writes one JSON report.

// Looks cards up, so this program holds the pool (`baylee-cards-pool`).
use baylee_cards_pool as _;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use baylee_cards::decks::{by_name, leader_of, may_lead_together};
use baylee_cards_dsl::Coverage;
use baylee_core::ids::CardIndex;
use baylee_train::deckgen;
use baylee_train::housedeck::HouseDeck;
use baylee_train::working::{Refusal, Working, repo_root};
use clap::Parser;
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};

#[derive(Parser)]
struct Args {
    /// The training manifest whose decks exist already (its splits and lists).
    #[arg(long)]
    manifest: PathBuf,
    /// Further manifests of decks that exist already (similarity only).
    #[arg(long)]
    existing: Vec<PathBuf>,
    /// Near-complete MTGO lists (`mtgo_near_complete_v1`).
    #[arg(long)]
    near_complete: PathBuf,
    /// Card names never to pick (held back for a cause this batch can't fix).
    #[arg(long)]
    exclude: Option<PathBuf>,
    /// Picks to make.
    #[arg(long, default_value_t = 40)]
    picks: usize,
    /// A new deck at or above this multiset Jaccard to a kept deck is dropped.
    #[arg(long, default_value_t = 0.85)]
    similarity: f64,
    /// Pick only cards that have a card file already (stub, partial, ...).
    #[arg(long)]
    in_pool_only: bool,
    /// Reader-gap audit input: `transcode-report --samples N` refusal lines.
    #[arg(long)]
    causes: Option<PathBuf>,
    /// The corpus script index (`data/script-index.json`: name to path).
    #[arg(long)]
    script_index: Option<PathBuf>,
    /// Report path.
    #[arg(long)]
    out: PathBuf,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    PreconNew,
    PreconUpgrade,
    Mtgo,
}

struct Deck {
    key: String,
    label: String,
    kind: Kind,
    format: String,
    /// Main deck and commanders, by index, with counts.
    cards: BTreeMap<u32, u64>,
    /// Not-working cards it needs.
    missing: BTreeSet<u32>,
    /// It holds a working held-out card, or its split is not train.
    out_of_train: Option<String>,
}

#[derive(Default)]
struct Sim {
    postings: BTreeMap<u32, Vec<(usize, u64)>>,
    sizes: Vec<u64>,
    ids: Vec<String>,
}

impl Sim {
    fn add(&mut self, id: &str, cards: &BTreeMap<u32, u64>) {
        let n = self.sizes.len();
        for (c, k) in cards {
            self.postings.entry(*c).or_default().push((n, *k));
        }
        self.sizes.push(cards.values().sum());
        self.ids.push(id.to_owned());
    }
    #[allow(clippy::cast_precision_loss)]
    fn maximum(&self, cards: &BTreeMap<u32, u64>, skip: &str) -> (f64, String) {
        let mut inter: BTreeMap<usize, u64> = BTreeMap::new();
        for (c, k) in cards {
            if let Some(p) = self.postings.get(c) {
                for (id, other) in p {
                    *inter.entry(*id).or_default() += (*k).min(*other);
                }
            }
        }
        let size: u64 = cards.values().sum();
        let mut best = (0.0, String::new());
        for (id, n) in inter {
            if self.ids[id] == skip {
                continue;
            }
            let j = n as f64 / (size + self.sizes[id] - n) as f64;
            if j > best.0 {
                best = (j, self.ids[id].clone());
            }
        }
        best
    }
}

struct Cards {
    working: Working,
    held: BTreeSet<CardIndex>,
    names: BTreeMap<String, Option<u32>>,
}

impl Cards {
    fn resolve(&mut self, name: &str) -> Option<u32> {
        if let Some(hit) = self.names.get(name) {
            return *hit;
        }
        let hit = by_name(name)
            .or_else(|| baylee_cards_index::row_by_name(name).map(|r| r.index))
            .or_else(|| {
                let front = name.split(" // ").next()?;
                by_name(front).or_else(|| baylee_cards_index::row_by_name(front).map(|r| r.index))
            })
            .map(CardIndex::get);
        self.names.insert(name.to_owned(), hit);
        hit
    }
    fn works(&self, c: u32) -> bool {
        self.working.cards.contains(&CardIndex::new(c))
    }
    fn status(&self, c: u32) -> &'static str {
        let idx = CardIndex::new(c);
        match self.working.check(idx) {
            Ok(()) => "working",
            Err(Refusal::NotInPool) => "not_in_pool",
            Err(Refusal::Untested) => "implemented_untested",
            Err(Refusal::NotImplemented) => {
                match baylee_cards::by_index(idx).map(|d| &d.coverage) {
                    Some(Coverage::Generated) => "generated",
                    Some(Coverage::Partial(_)) => "partial",
                    _ => "stub",
                }
            }
        }
    }
    fn held_now(&self, c: u32) -> bool {
        self.held.contains(&CardIndex::new(c))
    }
    fn held_if_working(c: u32) -> bool {
        !deckgen::held_out_cards([CardIndex::new(c)], 20).is_empty()
    }
    fn name(c: u32) -> String {
        baylee_cards::by_index(CardIndex::new(c)).map_or_else(
            || {
                baylee_cards_index::ROWS
                    .get(c as usize)
                    .map_or_else(|| format!("#{c}"), |r| r.name.to_owned())
            },
            |d| d.name().to_owned(),
        )
    }
}

fn read_json(path: &Path) -> Result<Value> {
    Ok(serde_json::from_slice(
        &fs::read(path).with_context(|| path.display().to_string())?,
    )?)
}

fn deck_cards(deck: &HouseDeck) -> BTreeMap<u32, u64> {
    let mut m = BTreeMap::new();
    for c in deck.deck.main.iter().chain(&deck.deck.commanders) {
        *m.entry(c.index.get()).or_default() += 1;
    }
    m
}

#[allow(clippy::too_many_lines)]
fn main() -> Result<()> {
    let args = Args::parse();
    let root = repo_root();
    let working = Working::scan(&root)?;
    let held = deckgen::held_out(&working, 20);
    eprintln!(
        "working {} cards, held out {}, hash {}",
        working.cards.len(),
        held.len(),
        working.hash()
    );
    let mut cards = Cards {
        working,
        held,
        names: BTreeMap::new(),
    };
    let exclude: BTreeSet<String> = match &args.exclude {
        Some(p) => fs::read_to_string(p)?
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(str::to_owned)
            .collect(),
        None => BTreeSet::new(),
    };

    // Existing decks: their splits and similarity.
    let manifest = read_json(&args.manifest)?;
    let mut sim = Sim::default();
    let mut splits: BTreeMap<String, String> = BTreeMap::new();
    let mut unreadable = 0;
    let mut existing_lists = vec![manifest.clone()];
    for p in &args.existing {
        existing_lists.push(read_json(p)?);
    }
    for m in &existing_lists {
        for e in m["decks"].as_array().into_iter().flatten() {
            let id = e["id"].as_str().unwrap_or("?").to_owned();
            splits.insert(id.clone(), e["split"].as_str().unwrap_or("?").to_owned());
            let path = e["path"].as_str().unwrap_or_default();
            let path = if Path::new(path).is_absolute() {
                PathBuf::from(path)
            } else {
                root.join(path)
            };
            match fs::read_to_string(&path)
                .map_err(anyhow::Error::from)
                .and_then(|t| HouseDeck::parse_any(&id, &t))
            {
                Ok(d) => sim.add(&id, &deck_cards(&d)),
                Err(_) => unreadable += 1,
            }
        }
    }
    eprintln!(
        "existing decks {} ({unreadable} unreadable)",
        sim.sizes.len()
    );

    let mut decks: Vec<Deck> = Vec::new();
    let mut blocked: BTreeMap<String, usize> = BTreeMap::new();
    let mut complete_not_in_manifest: Vec<String> = Vec::new();

    // Precons.
    let precon_root = root.join("data/decks/precon");
    let mut files: Vec<PathBuf> = Vec::new();
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
    for path in files {
        let key = path
            .strip_prefix(&precon_root)?
            .with_extension("")
            .to_string_lossy()
            .replace('\\', "/");
        let text = fs::read_to_string(&path)?;
        let name = text
            .lines()
            .find_map(|l| l.strip_prefix("# name: "))
            .unwrap_or(&key)
            .trim()
            .to_owned();
        let Ok(baylee_deckio::Import::Read { read, .. }) = baylee_deckio::import(&text) else {
            *blocked.entry("precon_unreadable".into()).or_default() += 1;
            continue;
        };
        let mut m: BTreeMap<u32, u64> = BTreeMap::new();
        let mut missing = BTreeSet::new();
        let mut cmd: Vec<u32> = Vec::new();
        let mut main_n = 0u64;
        let mut why: Option<String> = None;
        for card in &read.document.cards {
            use baylee_deckio::Zone;
            if !matches!(card.zone, Zone::Main | Zone::Commander) {
                continue;
            }
            let Some(c) = cards.resolve(&card.name) else {
                why = Some("not_in_ledger".into());
                continue;
            };
            *m.entry(c).or_default() += u64::from(card.count);
            if card.zone == Zone::Commander {
                cmd.push(c);
            } else {
                main_n += u64::from(card.count);
            }
            if !cards.works(c) {
                missing.insert(c);
            }
        }
        for &c in &cmd {
            if leader_of(CardIndex::new(c)).is_none_or(|l| !l.eligible())
                && cards.status(c) != "not_in_pool"
            {
                why = Some("invalid_leader".into());
            }
        }
        if cmd.len() == 2
            && let (Some(a), Some(b)) = (
                leader_of(CardIndex::new(cmd[0])),
                leader_of(CardIndex::new(cmd[1])),
            )
            && !may_lead_together(&a, &b)
        {
            why = Some("cannot_lead_together".into());
        }
        if let Some(w) = why {
            *blocked.entry(format!("precon_{w}")).or_default() += 1;
            continue;
        }
        let format = if !cmd.is_empty() {
            "commander"
        } else if main_n >= 60 {
            "constructed"
        } else {
            "starter_limited"
        };
        if missing.is_empty() {
            if splits.contains_key(&format!("precon/{key}")) {
                continue;
            }
            complete_not_in_manifest.push(key.clone());
        }
        let adapted = splits.get(&format!("adapted/{key}")).cloned();
        let (kind, out) = match adapted.as_deref() {
            None => (Kind::PreconNew, None),
            Some("train") => (Kind::PreconUpgrade, None),
            Some(s) => (Kind::PreconUpgrade, Some(format!("adapted_in_{s}"))),
        };
        decks.push(Deck {
            key: format!("precon/{key}"),
            label: format!("{name} [{key}]"),
            kind,
            format: format.to_owned(),
            cards: m,
            missing,
            out_of_train: out,
        });
    }

    // MTGO near-complete lists (mainboard).
    let near = read_json(&args.near_complete)?;
    let mut identities = BTreeSet::new();
    for d in near["decks"].as_array().context("decks")? {
        let fmt = d["source_format"].as_str().unwrap_or("?").to_owned();
        let mut m: BTreeMap<u32, u64> = BTreeMap::new();
        let mut missing = BTreeSet::new();
        let mut why = None;
        for row in d["main"].as_array().into_iter().flatten() {
            let n = row[0].as_str().unwrap_or_default();
            let k = row[1].as_u64().unwrap_or(0);
            let Some(c) = cards.resolve(n) else {
                why = Some("not_in_ledger");
                continue;
            };
            *m.entry(c).or_default() += k;
            if !cards.works(c) {
                missing.insert(c);
            }
        }
        if let Some(w) = why {
            *blocked.entry(format!("mtgo_{w}")).or_default() += 1;
            continue;
        }
        let pairs: Vec<(u64, u64)> = m.iter().map(|(c, k)| (u64::from(*c), *k)).collect();
        let hex =
            Sha256::digest(serde_json::to_vec(&pairs)?)
                .iter()
                .fold(String::new(), |mut s, b| {
                    let _ = write!(s, "{b:02x}");
                    s
                });
        if !identities.insert(hex.clone()) {
            *blocked.entry("mtgo_duplicate".into()).or_default() += 1;
            continue;
        }
        let bucket = u32::from_str_radix(&hex[..8], 16)? % 10;
        let out = match bucket {
            0 => Some("bucket_test".to_owned()),
            1 => Some("bucket_val".to_owned()),
            _ => None,
        };
        if missing.is_empty() {
            if splits.contains_key(&format!("mtgo/{fmt}/{}", &hex[..20])) {
                continue;
            }
            complete_not_in_manifest.push(format!("mtgo/{fmt}/{}", &hex[..20]));
        }
        decks.push(Deck {
            key: format!("mtgo/{fmt}/{}", &hex[..20]),
            label: d["name"].as_str().unwrap_or("?").to_owned(),
            kind: Kind::Mtgo,
            format: fmt,
            cards: m,
            missing,
            out_of_train: out,
        });
    }
    eprintln!("candidate decks {} blocked {blocked:?}", decks.len());

    // Train eligibility before any pick: no working held-out card, no
    // missing card that would be held out once it works, a train split.
    for d in &mut decks {
        if d.out_of_train.is_some() {
            continue;
        }
        if d.cards.keys().any(|&c| cards.held_now(c)) {
            d.out_of_train = Some("held_out_card".into());
        } else if d.missing.iter().any(|&c| Cards::held_if_working(c)) {
            d.out_of_train = Some("missing_card_held_out".into());
        } else if d.missing.iter().any(|&c| exclude.contains(&Cards::name(c))) {
            d.out_of_train = Some("excluded_card".into());
        }
    }
    let eligible: Vec<usize> = (0..decks.len())
        .filter(|&i| decks[i].out_of_train.is_none())
        .collect();
    let mut out_reasons: BTreeMap<String, usize> = BTreeMap::new();
    for d in &decks {
        if let Some(r) = &d.out_of_train {
            let k = format!("{:?}:{r}", d.kind);
            *out_reasons.entry(k).or_default() += 1;
        }
    }
    eprintln!("train-eligible {} out {out_reasons:?}", eligible.len());
    let mut by_missing: BTreeMap<String, usize> = BTreeMap::new();
    for &i in &eligible {
        let d = &decks[i];
        let n = d.missing.len();
        let bucket = if n >= 6 {
            "6+".to_owned()
        } else {
            n.to_string()
        };
        *by_missing
            .entry(format!("{:?}:{bucket}", d.kind))
            .or_default() += 1;
    }
    eprintln!("eligible by missing count {by_missing:?}");
    for &i in &eligible {
        let d = &decks[i];
        if d.kind != Kind::Mtgo && (1..=2).contains(&d.missing.len()) {
            let m: Vec<String> = d
                .missing
                .iter()
                .map(|&c| format!("{} ({})", Cards::name(c), cards.status(c)))
                .collect();
            eprintln!(
                "precon {:?} {} [{}]: {}",
                d.kind,
                d.label,
                d.format,
                m.join(", ")
            );
        }
    }

    // Single-card view: how many eligible decks each card appears in, and
    // how many it alone completes.
    let mut appears: BTreeMap<u32, (usize, usize)> = BTreeMap::new();
    for &i in &eligible {
        let d = &decks[i];
        for &c in &d.missing {
            let e = appears.entry(c).or_default();
            e.0 += 1;
            if d.missing.len() == 1 {
                e.1 += 1;
            }
        }
    }

    // Reader-gap audit: for each refusal cause, the cards it blocks and the
    // TRAIN decks those cards alone would complete.
    if let (Some(causes_path), Some(index_path)) = (&args.causes, &args.script_index) {
        let index: BTreeMap<String, String> = serde_json::from_slice(&fs::read(index_path)?)?;
        let by_path: BTreeMap<&str, &str> = index
            .iter()
            .map(|(n, p)| (p.as_str(), n.as_str()))
            .collect();
        let mut by_cause: BTreeMap<String, Vec<u32>> = BTreeMap::new();
        let mut scripts: BTreeMap<String, usize> = BTreeMap::new();
        for line in fs::read_to_string(causes_path)?.lines() {
            let Some(rest) = line.strip_prefix("--- refused (") else {
                continue;
            };
            let Some((cause, path)) = rest.rsplit_once("): ") else {
                continue;
            };
            *scripts.entry(cause.to_owned()).or_default() += 1;
            let rel = path.split("/cardsfolder/").nth(1).unwrap_or(path);
            let Some(name) = by_path.get(rel) else {
                continue;
            };
            if let Some(c) = cards.resolve(name) {
                by_cause.entry(cause.to_owned()).or_default().push(c);
            }
        }
        let mut rows = Vec::new();
        for (cause, list) in &by_cause {
            let set: BTreeSet<u32> = list.iter().copied().collect();
            let pool_stubs = set.iter().filter(|c| cards.status(**c) == "stub").count();
            let mut kept_local = Sim::default();
            let mut decks_done: BTreeMap<String, usize> = BTreeMap::new();
            let mut needed: BTreeMap<u32, usize> = BTreeMap::new();
            for &i in &eligible {
                let d = &decks[i];
                if d.missing.is_empty() || !d.missing.is_subset(&set) {
                    continue;
                }
                let skip = d.key.replace("precon/", "adapted/");
                if sim.maximum(&d.cards, &skip).0 >= args.similarity
                    || kept_local.maximum(&d.cards, "").0 >= args.similarity
                {
                    continue;
                }
                kept_local.add(&d.key, &d.cards);
                let k = match d.kind {
                    Kind::PreconNew => "precon_new",
                    Kind::PreconUpgrade => "precon_upgrade",
                    Kind::Mtgo => "mtgo",
                };
                *decks_done.entry(format!("{k}:{}", d.format)).or_default() += 1;
                for c in &d.missing {
                    *needed.entry(*c).or_default() += 1;
                }
            }
            let total: usize = decks_done.values().sum();
            let mut top: Vec<(u32, usize)> = needed.into_iter().collect();
            top.sort_by_key(|row| std::cmp::Reverse(row.1));
            rows.push(json!({
                "cause": cause, "scripts": scripts.get(cause).copied().unwrap_or(0),
                "pool_stubs": pool_stubs, "train_decks": total, "by_format": decks_done,
                "cards_in_those_decks": top.iter().take(8).map(|(c, n)| format!("{} ({n})", Cards::name(*c))).collect::<Vec<_>>(),
            }));
        }
        rows.sort_by(|a, b| {
            b["train_decks"]
                .as_u64()
                .cmp(&a["train_decks"].as_u64())
                .then(b["scripts"].as_u64().cmp(&a["scripts"].as_u64()))
        });
        fs::write(
            &args.out,
            serde_json::to_vec_pretty(&json!({"causes": rows}))?,
        )?;
        eprintln!("cause audit written");
        return Ok(());
    }

    // Greedy.
    let mut chosen: BTreeSet<u32> = BTreeSet::new();
    let mut accepted: BTreeSet<usize> = BTreeSet::new();
    let mut kept = sim;
    let mut picks = Vec::new();
    let mut cum = BTreeMap::<&str, usize>::new();
    // Round 0: decks that are complete already and would be admitted by a
    // re-import today, with no card work at all.
    let mut free: BTreeMap<String, usize> = BTreeMap::new();
    let mut free_arch: BTreeMap<String, usize> = BTreeMap::new();
    for &i in &eligible {
        let d = &decks[i];
        if !d.missing.is_empty() {
            continue;
        }
        let skip = d.key.replace("precon/", "adapted/");
        if kept.maximum(&d.cards, &skip).0 >= args.similarity {
            *free.entry("dropped_near_duplicate".into()).or_default() += 1;
            continue;
        }
        kept.add(&d.key, &d.cards);
        accepted.insert(i);
        *free
            .entry(format!("{:?}:{}", d.kind, d.format))
            .or_default() += 1;
        *free_arch.entry(archetype(d)).or_default() += 1;
    }
    eprintln!("free train decks (complete now): {free:?}\n  archetypes {free_arch:?}");
    for round in 0..args.picks {
        let mut best: Option<(usize, f64, u32, Vec<usize>)> = None;
        let mut cand: BTreeMap<u32, (Vec<usize>, f64)> = BTreeMap::new();
        for &i in &eligible {
            if accepted.contains(&i) {
                continue;
            }
            let d = &decks[i];
            let left: Vec<u32> = d.missing.difference(&chosen).copied().collect();
            if left.is_empty() {
                // Complete, but dropped as a near-duplicate of a kept deck.
                continue;
            }
            #[allow(clippy::cast_precision_loss)]
            let w = 1.0 / (1u64 << (left.len().min(20) - 1)) as f64;
            for &c in &left {
                if exclude.contains(&Cards::name(c))
                    || Cards::held_if_working(c)
                    || (args.in_pool_only && cards.status(c) == "not_in_pool")
                {
                    continue;
                }
                let e = cand.entry(c).or_default();
                e.1 += w;
                if left.len() == 1 {
                    e.0.push(i);
                }
            }
        }
        for (c, (completes, progress)) in cand {
            // Count completions that survive the similarity check, in order,
            // against what is kept already and each other.
            let mut local = Vec::new();
            let mut tmp_ids: Vec<&BTreeMap<u32, u64>> = Vec::new();
            for &i in &completes {
                let d = &decks[i];
                let skip = d.key.replace("precon/", "adapted/");
                let (j, _) = kept.maximum(&d.cards, &skip);
                if j >= args.similarity {
                    continue;
                }
                let dup = tmp_ids
                    .iter()
                    .any(|o| jaccard(o, &d.cards) >= args.similarity);
                if dup {
                    continue;
                }
                tmp_ids.push(&d.cards);
                local.push(i);
            }
            let score = local.len();
            // Most decks completed, then most progress, then the lower index.
            let better = best.as_ref().is_none_or(|(s, p, other, _)| {
                score
                    .cmp(s)
                    .then(progress.total_cmp(p))
                    .then(other.cmp(&c))
                    .is_gt()
            });
            if better {
                best = Some((score, progress, c, local));
            }
        }
        let Some((score, progress, c, local)) = best else {
            break;
        };
        chosen.insert(c);
        let mut by_fmt: BTreeMap<String, usize> = BTreeMap::new();
        let mut names = Vec::new();
        let mut arch: BTreeMap<String, usize> = BTreeMap::new();
        for &i in &local {
            let d = &decks[i];
            accepted.insert(i);
            kept.add(&d.key, &d.cards);
            let k = match d.kind {
                Kind::PreconNew => "precon_new",
                Kind::PreconUpgrade => "precon_upgrade",
                Kind::Mtgo => "mtgo",
            };
            *cum.entry(k).or_default() += 1;
            *by_fmt.entry(format!("{k}:{}", d.format)).or_default() += 1;
            *arch.entry(archetype(d)).or_default() += 1;
            names.push(format!("{} ({}, {})", d.label, d.format, archetype(d)));
        }
        let (app, alone) = appears.get(&c).copied().unwrap_or_default();
        let total: usize = cum.values().sum();
        eprintln!(
            "{:>2}. {:<40} {:<22} +{score:<3} cum {total:<4} progress {progress:.2} in {app} decks ({alone} alone)",
            round + 1,
            Cards::name(c),
            cards.status(c)
        );
        picks.push(json!({
            "rank": round + 1, "card": Cards::name(c), "index": c, "archetypes": arch,
            "status": cards.status(c), "completes": score, "progress": progress,
            "cumulative": total, "cumulative_by_kind": cum.clone(),
            "in_eligible_decks": app, "sole_missing_in": alone,
            "by_format": by_fmt, "decks": names,
        }));
    }

    // Top cards by appearance, for reference.
    let mut top: Vec<(u32, (usize, usize))> = appears.into_iter().collect();
    top.sort_by(|a, b| b.1.0.cmp(&a.1.0).then(a.0.cmp(&b.0)));
    let top: Vec<Value> = top
        .iter()
        .take(150)
        .map(|(c, (n, alone))| {
            json!({"card": Cards::name(*c), "status": cards.status(*c),
                   "held_if_working": Cards::held_if_working(*c),
                   "excluded": exclude.contains(&Cards::name(*c)),
                   "in_eligible_decks": n, "sole_missing_in": alone})
        })
        .collect();
    let report = json!({
        "working_hash": cards.working.hash(),
        "working_cards": cards.working.cards.len(),
        "candidate_decks": decks.len(),
        "train_eligible_decks": eligible.len(),
        "out_of_train": out_reasons,
        "blocked": blocked,
        "complete_not_in_manifest": complete_not_in_manifest.len(),
        "free_train_decks": free, "free_archetypes": free_arch,
        "picks": picks,
        "top_by_appearance": top,
    });
    fs::write(&args.out, serde_json::to_vec_pretty(&report)?)?;
    Ok(())
}

#[allow(clippy::cast_precision_loss)]
fn jaccard(a: &BTreeMap<u32, u64>, b: &BTreeMap<u32, u64>) -> f64 {
    let mut inter = 0u64;
    for (c, k) in a {
        if let Some(o) = b.get(c) {
            inter += (*k).min(*o);
        }
    }
    let sa: u64 = a.values().sum();
    let sb: u64 = b.values().sum();
    inter as f64 / (sa + sb - inter) as f64
}

/// The importer's archetype and tags for a deck, from the cards the pool has.
fn archetype(d: &Deck) -> String {
    let main: Vec<CardIndex> = d
        .cards
        .iter()
        .flat_map(|(c, k)| {
            std::iter::repeat_n(CardIndex::new(*c), usize::try_from(*k).unwrap_or(0))
        })
        .collect();
    let (arch, tags, colors) = baylee_cards::deckgen::strategy::classify_deck_strategy(&main, &[]);
    let tags: Vec<&str> = tags
        .iter()
        .map(String::as_str)
        .filter(|t| *t != arch && *t != "midrange")
        .collect();
    format!(
        "{arch} {} [{}]",
        baylee_cards::deckgen::strategy::color_string(colors),
        tags.join(",")
    )
}
