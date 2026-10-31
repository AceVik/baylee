//! Read pinned MTGO tournament archives without extracting paths or retaining identities.
pub mod similarity;
#[cfg(test)]
mod tests;
use super::cards::{Card, fold, registry_cards};
use crate::files;
use anyhow::{Context, Result, ensure};
use flate2::read::MultiGzDecoder;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use similarity::SimilarityIndex;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write,
    fs::File,
    path::PathBuf,
};
const REPO: &str = "https://github.com/fbettega/MTG_decklistcache";
const FORMATS: &[&str] = &[
    "premodern",
    "vintage",
    "legacy",
    "pauper",
    "modern",
    "standard",
    "pioneer",
];
const BASICS: &[&str] = &[
    "plains",
    "island",
    "swamp",
    "mountain",
    "forest",
    "wastes",
    "snow-covered plains",
    "snow-covered island",
    "snow-covered swamp",
    "snow-covered mountain",
    "snow-covered forest",
];
/// Options for an offline, pinned tournament corpus import.
pub struct Options {
    /// Directory containing source.tar.gz, revision.txt and LICENSE.
    pub source: PathBuf,
    /// Frozen native `deck_catalog4` export, including existing decks.
    pub cards: PathBuf,
    /// New external output directory.
    pub out: PathBuf,
    /// Maximum accepted lists per historical source format.
    pub max_per_format: u64,
    /// Reject a multiset Jaccard at or above this threshold.
    pub similarity: f64,
    /// Explicit first-game projection; discard all sideboard entries.
    pub mainboard_only: bool,
}
struct Deck {
    cards: BTreeMap<u64, u64>,
    names: BTreeMap<u64, String>,
    held: bool,
}
enum CardError {
    /// Every distinct card of the list the pool does not have.
    Unsupported(BTreeSet<String>),
    Invalid,
}
/// A deck missing at most this many distinct cards is near complete: the
/// report ranks the missing cards by how many such decks they would unlock.
const NEAR_COMPLETE: usize = 3;
fn field<'a>(value: &'a Value, pascal: &str, snake: &str) -> &'a Value {
    value
        .get(pascal)
        .or_else(|| value.get(snake))
        .unwrap_or(&Value::Null)
}
fn read_cards(
    rows: &Value,
    lookup: &BTreeMap<String, Card>,
) -> std::result::Result<Deck, CardError> {
    let empty = Vec::new();
    let rows = rows.as_array().unwrap_or(&empty);
    let mut result = Deck {
        cards: BTreeMap::new(),
        names: BTreeMap::new(),
        held: false,
    };
    let mut missing = BTreeSet::new();
    for row in rows {
        let name = field(row, "CardName", "card_name")
            .as_str()
            .unwrap_or_default();
        let count = field(row, "Count", "count")
            .as_u64()
            .filter(|n| (1..=100).contains(n))
            .ok_or(CardError::Invalid)?;
        // Read on past an unsupported card: the report counts all of them.
        let Some(card) = lookup.get(&fold(name)) else {
            missing.insert(name.to_owned());
            continue;
        };
        *result.cards.entry(card.id).or_default() += count;
        result.names.insert(card.id, card.name.clone());
        result.held |= card.held;
    }
    if missing.is_empty() {
        Ok(result)
    } else {
        Err(CardError::Unsupported(missing))
    }
}
fn bump(counts: &mut BTreeMap<String, u64>, key: &str) {
    *counts.entry(key.to_owned()).or_default() += 1;
}
struct Importer {
    root: PathBuf,
    revision: String,
    pool: Value,
    lookup: BTreeMap<String, Card>,
    similarity: SimilarityIndex,
    seen: BTreeSet<String>,
    counts: BTreeMap<String, u64>,
    /// Lists missing each card, over every tournament entry.
    unsupported: BTreeMap<String, u64>,
    /// Near-complete lists each missing card would complete or bring closer.
    unlocks: BTreeMap<String, u64>,
    /// Lists by how many distinct cards they miss (`5+` beyond four).
    missing_histogram: BTreeMap<String, u64>,
    per_format: BTreeMap<String, u64>,
    accepted: Vec<Value>,
    rejected: Vec<Value>,
}
/// Import supported complete lists from an external pinned archive, with exact deduplication.
///
/// # Errors
/// Rejects invalid catalogs, occupied outputs, malformed archives and failed writes.
pub fn import(options: &Options) -> Result<Value> {
    ensure!(
        options.similarity.is_finite() && (0.0..=1.0).contains(&options.similarity),
        "similarity threshold must be in [0,1]"
    );
    let registry = files::read_json(&options.cards)?;
    let existing = registry["existing_decks"]
        .as_array()
        .filter(|e| !e.is_empty())
        .context("An authoritative existing-corpus duplicate check is required")?;
    if let Some(parent) = options.out.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::create_dir(&options.out)?;
    let root = options.out.canonicalize()?;
    std::fs::create_dir(root.join("decks"))?;
    std::fs::copy(options.source.join("LICENSE"), root.join("LICENSE"))?;
    let lookup = registry_cards(&registry)?;
    let similarity = existing_similarity(existing)?;
    let revision = std::fs::read_to_string(options.source.join("revision.txt"))?
        .trim()
        .to_owned();
    let archive = options.source.join("source.tar.gz");
    let archive_sha = files::sha256(&archive)?;
    let mut importer = Importer {
        root,
        revision,
        pool: registry["working_pool_hash"].clone(),
        lookup,
        similarity,
        seen: BTreeSet::new(),
        counts: BTreeMap::new(),
        unsupported: BTreeMap::new(),
        unlocks: BTreeMap::new(),
        missing_histogram: BTreeMap::new(),
        per_format: BTreeMap::new(),
        accepted: Vec::new(),
        rejected: Vec::new(),
    };
    read_archive(&archive, &mut importer, options)?;
    importer.finish(&archive_sha, existing.len(), options)
}
fn existing_similarity(existing: &[Value]) -> Result<SimilarityIndex> {
    let mut similarity = SimilarityIndex::default();
    for deck in existing {
        let main = deck["main"]
            .as_object()
            .context("invalid existing mainboard")?
            .iter()
            .map(|(k, v)| {
                Ok((
                    k.parse::<u64>()?,
                    v.as_u64().context("invalid existing card count")?,
                ))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        similarity.add(&main);
    }
    Ok(similarity)
}
fn read_archive(
    archive: &std::path::Path,
    importer: &mut Importer,
    options: &Options,
) -> Result<()> {
    let mut archive = tar::Archive::new(MultiGzDecoder::new(File::open(archive)?));
    for entry in archive.entries()? {
        let mut entry = entry?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let name = entry.path()?.to_string_lossy().into_owned();
        if std::path::Path::new(&name)
            .extension()
            .is_none_or(|e| e != "json")
        {
            continue;
        }
        let Some((_, relative)) = name.split_once('/') else {
            continue;
        };
        if !relative.starts_with("Tournaments/MTGO/") {
            continue;
        }
        let base = relative.rsplit('/').next().unwrap_or_default();
        let Some(format) = FORMATS
            .iter()
            .copied()
            .find(|f| base.starts_with(&format!("{f}-")))
        else {
            continue;
        };
        if importer.per_format.get(format).copied().unwrap_or(0) >= options.max_per_format {
            continue;
        }
        let document = match serde_json::from_reader::<_, Value>(&mut entry) {
            Ok(v) => v,
            Err(error) => {
                bump(&mut importer.counts, "invalid_source_json");
                importer
                    .rejected
                    .push(json!({"path":relative,"reason":error.to_string()}));
                continue;
            }
        };
        bump(&mut importer.counts, "tournaments");
        let empty = Vec::new();
        for (number, raw) in field(&document, "Decks", "decks")
            .as_array()
            .unwrap_or(&empty)
            .iter()
            .enumerate()
        {
            importer.admit(
                options,
                format,
                relative,
                number,
                raw,
                field(&document, "Tournament", "tournament"),
            )?;
            if importer.per_format.get(format).copied().unwrap_or(0) >= options.max_per_format {
                break;
            }
        }
    }
    Ok(())
}
impl Importer {
    fn admit(
        &mut self,
        options: &Options,
        format: &str,
        relative: &str,
        number: usize,
        raw: &Value,
        tournament: &Value,
    ) -> Result<()> {
        bump(&mut self.counts, "decks_seen");
        let main = read_cards(field(raw, "Mainboard", "mainboard"), &self.lookup);
        let side = read_cards(
            if options.mainboard_only {
                &Value::Null
            } else {
                field(raw, "Sideboard", "sideboard")
            },
            &self.lookup,
        );
        let (main, side) = match (main, side) {
            (Ok(main), Ok(side)) => (main, side),
            (Err(CardError::Invalid), _) | (_, Err(CardError::Invalid)) => {
                bump(&mut self.counts, "invalid_count");
                return Ok(());
            }
            (main, side) => {
                let mut missing = BTreeSet::new();
                for part in [main, side] {
                    if let Err(CardError::Unsupported(names)) = part {
                        missing.extend(names);
                    }
                }
                self.record_missing(&missing);
                return Ok(());
            }
        };
        let main_count: u64 = main.cards.values().sum();
        let side_count: u64 = side.cards.values().sum();
        if !(60..=100).contains(&main_count) || side_count > 15 {
            bump(&mut self.counts, "invalid_size");
            return Ok(());
        }
        if main
            .names
            .values()
            .all(|n| BASICS.contains(&fold(n).as_str()))
        {
            bump(&mut self.counts, "all_basic_placeholder");
            return Ok(());
        }
        let pairs = main
            .cards
            .iter()
            .map(|(id, count)| (*id, *count))
            .collect::<Vec<_>>();
        let identity = files::hex(&Sha256::digest(serde_json::to_vec(&pairs)?));
        if !self.seen.insert(identity.clone()) {
            bump(&mut self.counts, "duplicate");
            return Ok(());
        }
        let overlap = self.similarity.maximum(&main.cards);
        if overlap >= options.similarity {
            bump(&mut self.counts, "near_duplicate_existing_or_new");
            return Ok(());
        }
        let held = main.held || side.held;
        let bucket = u32::from_str_radix(&identity[..8], 16)? % 10;
        let split = if held || bucket == 0 {
            "test"
        } else if bucket == 1 {
            "val"
        } else {
            "train"
        };
        let title = format!("MTGO {format} {}", &identity[..12]);
        let filename = self.root.join("decks").join(format!("{identity}.txt"));
        let source_ref = format!("{REPO}/blob/{}/{relative}", self.revision);
        let mut content = format!(
            "# source: {source_ref}, deck index {number}\n# attribution: fbettega/MTG_decklistcache; MTGO public tournament lists\n# licence: GPL-3.0 (see ../LICENSE)\n# changes: mainboard unchanged; sideboard {}; names normalized; player identity omitted\n[deck:{title}]\n",
            if options.mainboard_only {
                "omitted (first-game projection)"
            } else {
                "unchanged"
            }
        );
        for (card, count) in &main.cards {
            writeln!(content, "{count} {}", main.names[card])?;
        }
        if !side.cards.is_empty() {
            content.push_str("\n[sideboard]\n");
            for (card, count) in &side.cards {
                writeln!(content, "{count} {}", side.names[card])?;
            }
        }
        std::fs::write(&filename, content)?;
        self.accepted.push(json!({"id":format!("mtgo/{format}/{}",&identity[..20]),"name":title,"path":filename,"format":"constructed","source_format":format,"split":split,"holdout_tier":if held{"unseen_cards".to_owned()}else{format!("{split}_web_decks")},"contains_held_out_cards":held,"is_derived":options.mainboard_only,"transformation":if options.mainboard_only{"mainboard_only"}else{"none"},"source_key":"mtgo_decklistcache","source_ref":source_ref,"source_deck_index":number,"source_revision":self.revision,"source_date":field(tournament,"Date","date"),"license":"GPL-3.0","deck_hash":identity,"main_count":main_count,"sideboard_count":side_count,"distinct_cards":main.cards.len(),"max_preceding_jaccard":overlap,"working_pool_hash":self.pool,"archetype":"unclassified","strategy_confidence":"not_inferred"}));
        self.similarity.add(&main.cards);
        bump(&mut self.per_format, format);
        bump(&mut self.counts, "accepted");
        Ok(())
    }
    /// Counts a list the pool cannot play yet, by every card it misses.
    fn record_missing(&mut self, missing: &BTreeSet<String>) {
        bump(&mut self.counts, "unsupported_card");
        let bucket = if missing.len() > 4 {
            "5+".to_owned()
        } else {
            missing.len().to_string()
        };
        bump(&mut self.missing_histogram, &bucket);
        for name in missing {
            bump(&mut self.unsupported, name);
            if missing.len() <= NEAR_COMPLETE {
                bump(&mut self.unlocks, name);
            }
        }
    }
    fn finish(self, archive_sha: &str, existing: usize, options: &Options) -> Result<Value> {
        let mut splits = BTreeMap::new();
        for deck in &self.accepted {
            bump(
                &mut splits,
                deck["split"].as_str().context("invalid deck split")?,
            );
        }
        let now = time::OffsetDateTime::now_utc();
        let retrieved = format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:06}+00:00",
            now.year(),
            u8::from(now.month()),
            now.day(),
            now.hour(),
            now.minute(),
            now.second(),
            now.microsecond()
        );
        let manifest = json!({"source":REPO,"source_revision":self.revision,"source_archive_sha256":archive_sha,"retrieved_at":retrieved,"working_pool_hash":self.pool,"existing_decks_checked":existing,"max_jaccard_threshold":options.similarity,"counts_by_source_format":self.per_format,"counts_by_split":splits,"decks":self.accepted});
        files::atomic_json(&self.root.join("manifest.json"), &manifest)?;
        files::atomic_json(
            &self.root.join("import-report.json"),
            &json!({"counts":self.counts,
                "missing_card_histogram":self.missing_histogram,
                "near_complete_limit":NEAR_COMPLETE,
                "near_complete_unlocks":ranked(self.unlocks),
                "unsupported_card_lists":ranked(self.unsupported),
                "rejected_sources":self.rejected}),
        )?;
        Ok(manifest)
    }
}
/// Counts as `[name, count]` rows, the largest first.
fn ranked(counts: BTreeMap<String, u64>) -> Vec<(String, u64)> {
    let mut rows = counts.into_iter().collect::<Vec<_>>();
    rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    rows
}
