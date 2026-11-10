//! Which split of a deck manifest a seated deck belongs to, read from what a
//! game holds: by its key, else by its exact cards (main deck and commanders),
//! else `near`, the manifest deck sharing most of its distinct cards when that
//! share is at least [`NEAR`]. One reading for the audit (`seated_decks4`) and
//! for the guard every training conversion and collection runs ([`TrainOnly`]):
//! a training dataset holds no game that seated a deck the manifest puts
//! outside `train` (test, val, holdout2).

use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{BufRead as _, BufReader, Read as _, Seek as _, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::{Context as _, ensure};
use serde_json::{Value, json};

use crate::housedeck::HouseDeck;

/// The least share of distinct cards a `near` match holds in common.
pub const NEAR: f64 = 0.8;

/// A deck's identity across builds: its main deck and commanders, as sorted
/// card indices (indices are never reused, `docs/card-identity.md`).
pub type Cards = Vec<u32>;

/// The cards of `deck`.
#[must_use]
pub fn cards_of(deck: &HouseDeck) -> Cards {
    let mut cards: Cards = deck
        .deck
        .main
        .iter()
        .chain(&deck.deck.commanders)
        .map(|card| card.index.get())
        .collect();
    cards.sort_unstable();
    cards
}

/// The share of distinct cards two decks hold in common.
#[must_use]
pub fn share(mine: &BTreeSet<u32>, theirs: &BTreeSet<u32>) -> f64 {
    let common = mine.intersection(theirs).count();
    let all = mine.union(theirs).count().max(1);
    #[allow(clippy::cast_precision_loss, reason = "a few hundred cards")]
    let share = common as f64 / all as f64;
    share
}

/// One deck of the manifest.
pub struct Entry {
    /// `train`, `val`, `test`, `holdout2`, ...
    pub split: String,
    /// `constructed`, `commander`, ...
    pub format: String,
    /// It holds a card held out of training.
    pub held_out_cards: bool,
}

/// A manifest, by key and by cards.
pub struct Book {
    /// Every deck, by key.
    pub entries: BTreeMap<String, Entry>,
    by_cards: BTreeMap<Cards, String>,
    distinct: BTreeMap<String, BTreeSet<u32>>,
    /// The decks whose list could not be read on this build: known by key only.
    pub unreadable: Vec<String>,
}

impl Book {
    /// The manifest `manifest`, its deck paths read from `root`.
    #[must_use]
    pub fn read(manifest: &Value, root: &Path) -> Self {
        let mut book = Self {
            entries: BTreeMap::new(),
            by_cards: BTreeMap::new(),
            distinct: BTreeMap::new(),
            unreadable: Vec::new(),
        };
        for entry in manifest["decks"].as_array().into_iter().flatten() {
            let Some(id) = entry["id"].as_str() else {
                continue;
            };
            book.entries.insert(
                id.to_owned(),
                Entry {
                    split: entry["split"].as_str().unwrap_or("?").to_owned(),
                    format: entry["format"].as_str().unwrap_or("?").to_owned(),
                    held_out_cards: entry["contains_held_out_cards"].as_bool() == Some(true),
                },
            );
            let deck = entry["path"]
                .as_str()
                .context("no path")
                .and_then(|path| Ok(std::fs::read_to_string(root.join(path))?))
                .and_then(|text| HouseDeck::parse_any(id, &text));
            match deck {
                Ok(deck) => {
                    let cards = cards_of(&deck);
                    book.distinct
                        .insert(id.to_owned(), cards.iter().copied().collect());
                    book.by_cards.entry(cards).or_insert_with(|| id.to_owned());
                }
                Err(_) => book.unreadable.push(id.to_owned()),
            }
        }
        book
    }

    /// How many decks are known by their cards.
    #[must_use]
    pub fn by_cards(&self) -> usize {
        self.by_cards.len()
    }

    /// The split of the deck keyed `id`.
    #[must_use]
    pub fn split(&self, id: &str) -> Option<&str> {
        self.entries.get(id).map(|entry| entry.split.as_str())
    }

    /// The manifest deck sharing most distinct cards with `cards`, and the share.
    fn nearest(&self, cards: &Cards) -> Option<(&str, f64)> {
        let mine: BTreeSet<u32> = cards.iter().copied().collect();
        self.distinct
            .iter()
            .map(|(id, theirs)| (id.as_str(), share(&mine, theirs)))
            .max_by(|a, b| a.1.total_cmp(&b.1))
    }

    /// Which manifest deck `seat` is, and how that was read. `cache` keeps
    /// the near readings of lists already seen.
    pub fn identify(&self, seat: &Seat, near: f64, cache: &mut BTreeMap<Cards, Match>) -> Match {
        if let Some(key) = &seat.key
            && self.entries.contains_key(key)
        {
            // The same key in another corpus may hold another list: say how close.
            let how = match (&seat.cards, self.distinct.get(key)) {
                (Some(cards), _) if self.by_cards.get(cards) == Some(key) => "key+cards".to_owned(),
                (Some(cards), Some(theirs)) => {
                    format!("key {:.2}", share(&cards.iter().copied().collect(), theirs))
                }
                _ => "key".to_owned(),
            };
            return Match {
                id: Some(key.clone()),
                how,
            };
        }
        let Some(cards) = &seat.cards else {
            return Match {
                id: None,
                how: "no cards".into(),
            };
        };
        if let Some(id) = self.by_cards.get(cards) {
            return Match {
                id: Some(id.clone()),
                how: "cards".into(),
            };
        }
        cache
            .entry(cards.clone())
            .or_insert_with(|| match self.nearest(cards) {
                Some((id, share)) if share >= near => Match {
                    id: Some(id.to_owned()),
                    how: format!("near {share:.2}"),
                },
                _ => Match {
                    id: None,
                    how: "unmatched".into(),
                },
            })
            .clone()
    }
}

/// One seat of one game: what names it and what it held.
pub struct Seat {
    /// The deck's key, where the game or its run names it.
    pub key: Option<String>,
    /// The deck's cards, where the record holds them.
    pub cards: Option<Cards>,
}

/// How a seated deck was matched, and to which manifest deck.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Match {
    /// The manifest deck, if any.
    pub id: Option<String>,
    /// `key`, `key+cards`, `key 0.63`, `cards`, `near 0.91`, `unmatched`.
    pub how: String,
}

/// Every seat's cards in a record's header line (`preset.seats[].deck` and
/// `commanders`).
///
/// # Errors
/// A line that is not a record's header.
pub fn header_seats(header: &Value) -> anyhow::Result<Vec<Cards>> {
    ensure!(
        header["kind"] == "header",
        "the record does not begin with its header"
    );
    Ok(header["preset"]["seats"]
        .as_array()
        .context("a header without seats")?
        .iter()
        .map(|seat| {
            let mut cards: Cards = ["deck", "commanders"]
                .iter()
                .flat_map(|part| seat[*part].as_array().into_iter().flatten())
                .filter_map(|entry| entry["card"].as_u64())
                .map(|card| u32::try_from(card).unwrap_or(u32::MAX))
                .collect();
            cards.sort_unstable();
            cards
        })
        .collect())
}

/// The header seats of the game at `offset`, `len` of the gzip shard `shard`,
/// reading its first line only.
///
/// # Errors
/// An unreadable shard or a member that does not begin with a header.
pub fn record_seats(shard: &Path, offset: u64, len: u64) -> anyhow::Result<Vec<Cards>> {
    let mut file = File::open(shard).with_context(|| format!("opening {}", shard.display()))?;
    file.seek(SeekFrom::Start(offset))?;
    let mut line = String::new();
    BufReader::new(flate2::read::GzDecoder::new(file.take(len))).read_line(&mut line)?;
    header_seats(&serde_json::from_str(&line)?)
        .with_context(|| format!("{} at {offset}", shard.display()))
}

/// The keys a run's own deck list (`run.json` `decks[]{key, cards}`, as eval4
/// writes it) gives the card lists its games seat.
#[must_use]
pub fn run_deck_keys(run_json: &Value) -> BTreeMap<Cards, String> {
    run_json["decks"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|deck| {
            let key = deck["key"].as_str()?;
            let mut cards: Cards = deck["cards"]
                .as_array()?
                .iter()
                .filter_map(Value::as_u64)
                .map(|card| u32::try_from(card).unwrap_or(u32::MAX))
                .collect();
            cards.sort_unstable();
            Some((cards, key.to_owned()))
        })
        .collect()
}

/// The deck files (canonical paths) `manifest` lists in another split than
/// `split`. A house deck file (`data/decks/victory.txt`, keyed `victory` when
/// loaded by name) is the same deck as its manifest entry (`house/victory`),
/// so the split that keeps the entry out of a pool keeps the file out too.
#[must_use]
pub fn files_outside(manifest: &Value, root: &Path, split: &str) -> BTreeSet<PathBuf> {
    if split.is_empty() || split == "all" {
        return BTreeSet::new();
    }
    manifest["decks"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|entry| entry["split"].as_str() != Some(split))
        .filter_map(|entry| entry["path"].as_str())
        .filter_map(|path| std::fs::canonicalize(root.join(path)).ok())
        .collect()
}

/// The house deck files the repository's own manifest
/// (`data/decks/manifest.v4.json`) keeps out of train: what a collection that
/// loads house decks by name (`league`, `selfplay --decks`) may not seat.
///
/// # Errors
/// When that manifest cannot be read.
pub fn house_files_outside_train() -> anyhow::Result<BTreeSet<PathBuf>> {
    let root = crate::working::repo_root();
    let path = root.join("data/decks/manifest.v4.json");
    let manifest: Value = serde_json::from_slice(
        &std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?,
    )?;
    Ok(files_outside(&manifest, &root, "train"))
}

/// Refuses the house deck `key` at `path` when `outside` (from
/// [`house_files_outside_train`]) holds it: training seats train decks only.
///
/// # Errors
/// The deck named, when it is held out of train.
pub fn ensure_train_house_deck(
    key: &str,
    path: &Path,
    outside: &BTreeSet<PathBuf>,
) -> anyhow::Result<()> {
    ensure!(
        !std::fs::canonicalize(path).is_ok_and(|path| outside.contains(&path)),
        "house deck {key} ({}) is held out of the train split by data/decks/manifest.v4.json; \
         a training collection seats train decks only",
        path.display()
    );
    Ok(())
}

/// The guard of a training dataset or collection: no deck the manifest puts
/// outside `train`, read by key, cards or near.
pub struct TrainOnly {
    book: Book,
    path: PathBuf,
    sha256: String,
    cache: Mutex<BTreeMap<Cards, Match>>,
}

impl TrainOnly {
    /// The manifest at `path`, its decks read from `root`.
    ///
    /// # Errors
    /// An unreadable manifest, or one without decks.
    pub fn load(path: &Path, root: &Path) -> anyhow::Result<Self> {
        let bytes = std::fs::read(path)
            .with_context(|| format!("reading the train manifest {}", path.display()))?;
        let manifest: Value = serde_json::from_slice(&bytes)?;
        ensure!(
            manifest["decks"]
                .as_array()
                .is_some_and(|decks| !decks.is_empty()),
            "{}: a manifest without decks",
            path.display()
        );
        Ok(Self::new(
            Book::read(&manifest, root),
            path.to_path_buf(),
            baylee_train_ops::files::sha256(path)?,
        ))
    }

    fn new(book: Book, path: PathBuf, sha256: String) -> Self {
        Self {
            book,
            path,
            sha256,
            cache: Mutex::new(BTreeMap::new()),
        }
    }

    /// Why one of `seats` may not be trained on, naming the deck, its split
    /// and how it was read; `None` when every seat is a train deck or no
    /// deck of the manifest.
    ///
    /// # Panics
    /// When another thread panicked while reading a deck (a poisoned cache).
    #[must_use]
    pub fn refusal(&self, seats: &[Seat]) -> Option<String> {
        let mut cache = self.cache.lock().expect("deck cache lock");
        seats.iter().find_map(|seat| {
            let found = self.book.identify(seat, NEAR, &mut cache);
            let id = found.id?;
            let split = self.book.split(&id)?;
            (split != "train").then(|| {
                format!(
                    "seats deck {id}, which {} puts in the {split} split (read by {}); \
                     training data holds train decks only",
                    self.path.display(),
                    found.how
                )
            })
        })
    }

    /// What a dataset's `meta.json` says of the guard.
    #[must_use]
    pub fn metadata(&self) -> Value {
        json!({"manifest": self.path, "manifest_sha256": self.sha256,
            "rule": "no game seated a deck the manifest puts outside train, \
                     read by key, exact cards or near (shared distinct cards)",
            "near": NEAR})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::working::repo_root;

    fn cards(path: &str) -> Cards {
        let text = std::fs::read_to_string(repo_root().join(path)).unwrap();
        cards_of(&HouseDeck::parse_any("x", &text).unwrap())
    }

    fn guard() -> TrainOnly {
        let manifest = json!({"decks": [
            {"id": "eval/azorius-control", "path": "data/decks/eval/azorius-control.txt",
             "split": "holdout2", "format": "constructed"},
            {"id": "eval/red-burn", "path": "data/decks/eval/red-burn.txt",
             "split": "train", "format": "constructed"},
        ]});
        TrainOnly::new(
            Book::read(&manifest, &repo_root()),
            PathBuf::from("m.json"),
            String::new(),
        )
    }

    /// A deck outside train is refused by its key, by its cards under
    /// another key, and by most of its cards; a train deck and a deck the
    /// manifest does not know pass.
    #[test]
    fn only_train_decks_and_strangers_pass() {
        let guard = guard();
        let azorius = cards("data/decks/eval/azorius-control.txt");
        let burn = cards("data/decks/eval/red-burn.txt");
        let seat = |key: Option<&str>, cards: Option<Cards>| Seat {
            key: key.map(str::to_owned),
            cards,
        };
        assert_eq!(
            guard.refusal(&[seat(Some("eval/red-burn"), None), seat(None, Some(burn))]),
            None
        );
        assert_eq!(
            guard.refusal(&[seat(Some("gen-con-001"), Some(vec![1, 2, 3]))]),
            None
        );
        let by_key = guard
            .refusal(&[seat(Some("eval/azorius-control"), None)])
            .unwrap();
        assert!(
            by_key.contains("eval/azorius-control") && by_key.contains("holdout2"),
            "{by_key}"
        );
        let by_cards = guard
            .refusal(&[seat(Some("house-002"), Some(azorius.clone()))])
            .unwrap();
        assert!(by_cards.contains("read by cards"), "{by_cards}");
        let mut cut = azorius;
        cut.remove(0);
        cut.push(u32::MAX - 1);
        cut.sort_unstable();
        let near = guard.refusal(&[seat(None, Some(cut))]).unwrap();
        assert!(near.contains("read by near"), "{near}");
    }

    /// The repository's manifest holds Victory and the other house decks out
    /// of train; a collection loading house decks by name refuses them.
    #[test]
    fn a_held_out_house_deck_is_refused_by_name() {
        let outside = house_files_outside_train().unwrap();
        let dir = crate::housedeck::dir();
        let error = ensure_train_house_deck("victory", &dir.join("victory.txt"), &outside)
            .unwrap_err()
            .to_string();
        assert!(error.contains("victory"), "{error}");
        let manifest = json!({"decks": [
            {"id": "house/victory", "path": "data/decks/victory.txt", "split": "test"},
            {"id": "house/breya", "path": "data/decks/breya.txt", "split": "train"},
        ]});
        let root = repo_root();
        let held = files_outside(&manifest, &root, "train");
        assert!(ensure_train_house_deck("breya", &dir.join("breya.txt"), &held).is_ok());
        assert!(ensure_train_house_deck("victory", &dir.join("victory.txt"), &held).is_err());
        assert!(files_outside(&manifest, &root, "all").is_empty());
    }

    #[test]
    fn a_header_gives_every_seats_cards() {
        let header = json!({"kind": "header", "preset": {"seats": [
            {"deck": [{"card": 5}, {"card": 3}], "commanders": [{"card": 9}]},
            {"deck": [{"card": 1}], "commanders": []},
        ]}});
        assert_eq!(header_seats(&header).unwrap(), [vec![3, 5, 9], vec![1]]);
        assert!(header_seats(&json!({"kind": "input"})).is_err());
        let run = json!({"decks": [{"key": "a/b", "cards": [7, 2]}]});
        assert_eq!(
            run_deck_keys(&run),
            BTreeMap::from([(vec![2, 7], "a/b".to_owned())])
        );
    }
}
