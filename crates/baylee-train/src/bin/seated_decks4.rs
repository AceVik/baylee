//! Which decks recorded games seated, by the split a deck manifest puts them
//! in: read from what each game holds, never from a run's flags.
//!
//! A run is a directory with a `games.jsonl` (an `eval4 --record-out` run, a
//! converted language-model run, a v5 teacher run). For each game its record's
//! header gives every seat's cards (`preset.seats[].deck` and `commanders`),
//! and the game's row may name the decks (`decks`: keys, or objects with a
//! `key`). A seated deck is the manifest's by its key, else by its cards (the
//! same main deck and commanders), else, marked `near`, the manifest deck
//! sharing most of its distinct cards when that is at least `--near`; else it
//! is unmatched (a generated deck, a deck of another corpus).
//!
//! ```text
//! seated_decks4 --manifest manifest.v4.json --root RUNS --run RUN --out seated.json \
//!     --clean-out clean.json
//! ```
//!
//! `--clean-out` writes, per split and format, the manifest decks no game of
//! the runs seated, by key, card or near match; `--pools-dir` writes copies
//! of the manifest holding only such decks, one per pool, for `eval4
//! --decks-manifest`. A key match says how many of its distinct cards the
//! seated list shares with the manifest's (`key 0.63`): another corpus may
//! hold another list under the same key.

// Looks cards up, so this program holds the pool (`baylee-cards-pool`).
use baylee_cards_pool as _;

use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{BufRead as _, BufReader};
use std::path::{Path, PathBuf};

use anyhow::{Context as _, ensure};
use baylee_train::decksplit::{Book, Cards, Match, Seat, record_seats, run_deck_keys};
use baylee_train::working::repo_root;
use clap::Parser;
use serde_json::{Map, Value, json};

#[derive(Parser)]
#[command(about = "Which decks recorded games seated, by manifest split")]
struct Args {
    /// The deck manifest whose splits are asked about (`manifest.v4.json`).
    #[arg(long)]
    manifest: PathBuf,
    /// A run directory (with `games.jsonl`).
    #[arg(long)]
    run: Vec<PathBuf>,
    /// Every run directory below this one.
    #[arg(long)]
    root: Vec<PathBuf>,
    /// An eval4 summary (`--out`) whose games left no record, only their
    /// decks: `deal_details[].decks`, by key. The tactician's window files
    /// (`--tactician-out`) are read this way, through their run's summary.
    #[arg(long)]
    summary: Vec<PathBuf>,
    /// The least share of distinct cards a `near` match holds in common.
    #[arg(long, default_value_t = 0.8)]
    near: f64,
    /// The report, every run's seated decks, as JSON.
    #[arg(long)]
    out: PathBuf,
    /// The manifest decks no game of the runs seated, by split and format.
    #[arg(long)]
    clean_out: Option<PathBuf>,
    /// Writes here, per pool of never-seated decks, a copy of the manifest
    /// holding only that pool's entries, unchanged (`manifest-POOL.json`):
    /// `clean` (test and val decks without held-out cards), `held` (test
    /// decks with held-out cards), `unseen-train` (train decks).
    #[arg(long)]
    pools_dir: Option<PathBuf>,
}

/// The pool a never-seated deck of `split` falls in.
fn pool(split: &str, held_out_cards: bool) -> Option<&'static str> {
    match (split, held_out_cards) {
        ("test" | "val", false) => Some("clean"),
        ("test", true) => Some("held"),
        ("train", _) => Some("unseen-train"),
        _ => None,
    }
}

/// A copy of `manifest` with only the entries `keep` names, each unchanged.
fn manifest_of(manifest: &Value, keep: &BTreeSet<&str>) -> Value {
    let mut copy = manifest.clone();
    if let Some(decks) = copy["decks"].as_array_mut() {
        decks.retain(|entry| entry["id"].as_str().is_some_and(|id| keep.contains(id)));
    }
    copy
}

/// The seats of a game's record header, from its shard (`row`'s `shard`,
/// `offset`, `len`); `None` for a row that names no record.
fn header_seats(run: &Path, row: &Value) -> anyhow::Result<Option<Vec<Cards>>> {
    let (Some(shard), Some(offset), Some(len)) = (
        row["shard"].as_str(),
        row["offset"].as_u64(),
        row["len"].as_u64(),
    ) else {
        return Ok(None);
    };
    record_seats(&run.join("records").join(shard), offset, len).map(Some)
}

/// The keys a game's row gives its seats, if it gives any.
fn row_keys(row: &Value) -> Vec<Option<String>> {
    row["decks"]
        .as_array()
        .map(|decks| {
            decks
                .iter()
                .map(|deck| {
                    deck.as_str()
                        .or_else(|| deck["key"].as_str())
                        .map(str::to_owned)
                })
                .collect()
        })
        .unwrap_or_default()
}

/// What one run seated.
#[derive(Default)]
struct Seated {
    games: u64,
    unread: u64,
    /// Manifest key, or the `unmatched` bucket → (how, games).
    decks: BTreeMap<String, (String, u64)>,
    games_with: BTreeMap<String, u64>,
}

fn read_run(run: &Path, book: &Book, near: f64) -> anyhow::Result<(Seated, Value)> {
    let meta: Value = std::fs::read(run.join("run.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or(Value::Null);
    // The run's own deck list names a header's cards (eval4: `decks[]{key, cards}`).
    let named = run_deck_keys(&meta);
    let file = File::open(run.join("games.jsonl"))?;
    let mut seated = Seated::default();
    let mut cache = BTreeMap::new();
    for line in BufReader::new(file).lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let row: Value = serde_json::from_str(&line)?;
        seated.games += 1;
        let headers = header_seats(run, &row).unwrap_or(None);
        let keys = row_keys(&row);
        let count = headers
            .as_ref()
            .map_or(keys.len(), Vec::len)
            .max(keys.len());
        if count == 0 {
            seated.unread += 1;
            continue;
        }
        let seats = (0..count)
            .map(|index| {
                let cards = headers.as_ref().and_then(|seats| seats.get(index)).cloned();
                let key = keys
                    .get(index)
                    .cloned()
                    .flatten()
                    .or_else(|| cards.as_ref().and_then(|cards| named.get(cards).cloned()));
                Seat { key, cards }
            })
            .collect();
        seated.game(book, seats, near, &mut cache);
    }
    Ok((seated, meta))
}

impl Seated {
    /// One game's seats, each matched to the manifest.
    fn game(
        &mut self,
        book: &Book,
        seats: Vec<Seat>,
        near: f64,
        cache: &mut BTreeMap<Cards, Match>,
    ) {
        let mut splits = BTreeSet::new();
        for seat in seats {
            let found = book.identify(&seat, near, cache);
            let (name, split) = match &found.id {
                Some(id) => (
                    id.clone(),
                    book.entries
                        .get(id)
                        .map_or("?".to_owned(), |entry| entry.split.clone()),
                ),
                None => (
                    format!("unmatched:{}", seat.key.unwrap_or_else(|| "?".into())),
                    "unmatched".to_owned(),
                ),
            };
            let slot = self.decks.entry(name).or_insert((found.how, 0));
            slot.1 += 1;
            splits.insert(split);
        }
        for split in splits {
            *self.games_with.entry(split).or_default() += 1;
        }
    }
}

/// What an eval4 summary's deals seated, by key (`deal_details[].decks`).
fn read_summary(path: &Path, book: &Book, near: f64) -> anyhow::Result<(Seated, Value)> {
    let summary: Value = serde_json::from_slice(&std::fs::read(path)?)?;
    let mut seated = Seated::default();
    let mut cache = BTreeMap::new();
    for deal in summary["deal_details"]
        .as_array()
        .context("a summary without deal_details")?
    {
        seated.games += 1;
        let seats: Vec<Seat> = deal["decks"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|key| Seat {
                key: key.as_str().map(str::to_owned),
                cards: None,
            })
            .collect();
        if seats.is_empty() {
            seated.unread += 1;
            continue;
        }
        seated.game(book, seats, near, &mut cache);
    }
    let meta = json!({"deck_split": summary["deck_split"], "decks_manifest": summary["decks_manifest"],
        "training_data": Value::Null, "numbers_only": summary["numbers_only"],
        "collection_contract": summary["collection_contract"]});
    Ok((seated, meta))
}

/// How deep below a `--root` runs are looked for.
const DEPTH: usize = 6;

/// Every directory below `root`, at most `depth` down, holding a
/// `games.jsonl`; a run's own directories are not searched further.
fn runs_below(root: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    if root.join("games.jsonl").is_file() {
        out.push(root.to_path_buf());
        return;
    }
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    if depth == 0 {
        return;
    }
    let mut dirs: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    dirs.sort();
    for dir in dirs {
        runs_below(&dir, depth - 1, out);
    }
}

fn report(run: &Path, seated: &Seated, meta: &Value, book: &Book) -> Value {
    let mut by_split: BTreeMap<String, (u64, u64)> = BTreeMap::new();
    let mut listed: BTreeMap<String, Map<String, Value>> = BTreeMap::new();
    for (name, (how, games)) in &seated.decks {
        let split = book
            .entries
            .get(name)
            .map_or("unmatched".to_owned(), |entry| entry.split.clone());
        let tally = by_split.entry(split.clone()).or_default();
        tally.0 += 1;
        tally.1 += games;
        if split != "train" {
            listed
                .entry(split)
                .or_default()
                .insert(name.clone(), json!({"seat_games": games, "how": how}));
        }
    }
    json!({
        "run": run,
        "games": seated.games,
        "games_without_decks": seated.unread,
        "games_with": seated.games_with,
        "by_split": by_split.iter().map(|(split, (decks, games))|
            (split.clone(), json!({"decks": decks, "seat_games": games}))).collect::<Map<_, _>>(),
        "not_train": listed,
        "run_json": {
            "deck_split": meta["deck_split"], "decks_manifest": meta["decks_manifest"],
            "training_data": meta["training_data"], "numbers_only": meta["numbers_only"],
            "purpose": meta["collection_contract"]["purpose"],
            "source_collector": meta["source_collector"],
        },
    })
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let manifest: Value = serde_json::from_slice(
        &std::fs::read(&args.manifest)
            .with_context(|| format!("reading {}", args.manifest.display()))?,
    )?;
    let book = Book::read(&manifest, &repo_root());
    eprintln!(
        "[seated_decks4] {} manifest decks, {} by cards, {} unreadable",
        book.entries.len(),
        book.by_cards(),
        book.unreadable.len()
    );
    let mut runs = args.run.clone();
    for root in &args.root {
        runs_below(root, DEPTH, &mut runs);
    }
    ensure!(
        !runs.is_empty() || !args.summary.is_empty(),
        "no run directory or summary to read"
    );
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut reports = Vec::new();
    println!(
        "| run | games | train decks | val decks | test decks | unmatched | games with val | games with test |"
    );
    println!("| --- | --- | --- | --- | --- | --- | --- | --- |");
    let sources = runs
        .iter()
        .map(|run| (run, false))
        .chain(args.summary.iter().map(|summary| (summary, true)));
    for (run, summary) in sources {
        let read = if summary {
            read_summary(run, &book, args.near)
        } else {
            read_run(run, &book, args.near)
        };
        let (seated, meta) = match read {
            Ok(read) => read,
            Err(error) => {
                eprintln!("[seated_decks4] {}: {error:#}", run.display());
                reports.push(json!({"run": run, "error": format!("{error:#}")}));
                continue;
            }
        };
        seen.extend(seated.decks.keys().cloned());
        let row = report(run, &seated, &meta, &book);
        let decks = |split: &str| row["by_split"][split]["decks"].as_u64().unwrap_or(0);
        let with = |split: &str| row["games_with"][split].as_u64().unwrap_or(0);
        println!(
            "| {} | {} | {} | {} | {} | {} | {} | {} |",
            run.display(),
            seated.games,
            decks("train"),
            decks("val"),
            decks("test"),
            decks("unmatched"),
            with("val"),
            with("test"),
        );
        reports.push(row);
    }
    write_never_seated(&args, &book, &manifest, &runs, &seen)?;
    baylee_train_ops::files::atomic_json(
        &args.out,
        &json!({"manifest": args.manifest, "near": args.near,
            "unreadable_manifest_decks": book.unreadable, "runs": reports}),
    )
}

/// The manifest decks no game seated (`seen` holds those some game did):
/// counted, and with `--clean-out` and `--pools-dir` written.
fn write_never_seated(
    args: &Args,
    book: &Book,
    manifest: &Value,
    runs: &[PathBuf],
    seen: &BTreeSet<String>,
) -> anyhow::Result<()> {
    let mut clean: BTreeMap<String, BTreeMap<String, Vec<&str>>> = BTreeMap::new();
    for (id, entry) in &book.entries {
        if !seen.contains(id) {
            let format = if entry.held_out_cards {
                format!("{} (held-out cards)", entry.format)
            } else {
                entry.format.clone()
            };
            clean
                .entry(entry.split.clone())
                .or_default()
                .entry(format)
                .or_default()
                .push(id);
        }
    }
    let counts: BTreeMap<&String, BTreeMap<&String, usize>> = clean
        .iter()
        .map(|(split, formats)| {
            (
                split,
                formats.iter().map(|(f, ids)| (f, ids.len())).collect(),
            )
        })
        .collect();
    eprintln!(
        "[seated_decks4] never seated: {}",
        serde_json::to_string(&counts)?
    );
    if let Some(path) = &args.clean_out {
        baylee_train_ops::files::atomic_json(
            path,
            &json!({"manifest": args.manifest, "runs": runs, "summaries": args.summary, "near": args.near,
                "counts": counts, "never_seated": clean}),
        )?;
    }
    if let Some(dir) = &args.pools_dir {
        std::fs::create_dir_all(dir)?;
        let mut pools: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        for (id, entry) in &book.entries {
            if !seen.contains(id)
                && let Some(name) = pool(&entry.split, entry.held_out_cards)
            {
                pools.entry(name).or_default().insert(id);
            }
        }
        for (name, keep) in &pools {
            let copy = manifest_of(manifest, keep);
            baylee_train_ops::files::atomic_json(
                &dir.join(format!("manifest-{name}.json")),
                &copy,
            )?;
            eprintln!("[seated_decks4] pool {name}: {} decks", keep.len());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_train::decksplit::cards_of;
    use baylee_train::housedeck::HouseDeck;

    fn book() -> Book {
        let manifest = json!({"decks": [
            {"id": "eval/azorius-control", "path": "data/decks/eval/azorius-control.txt",
             "split": "test", "format": "constructed"},
            {"id": "eval/red-burn", "path": "data/decks/eval/red-burn.txt",
             "split": "train", "format": "constructed"},
        ]});
        Book::read(&manifest, &repo_root())
    }

    fn cards(path: &str) -> Cards {
        let text = std::fs::read_to_string(repo_root().join(path)).unwrap();
        cards_of(&HouseDeck::parse_any("x", &text).unwrap())
    }

    #[test]
    fn a_deck_is_known_by_its_key_then_its_cards_then_most_of_its_cards() {
        let book = book();
        let mut cache = BTreeMap::new();
        let azorius = cards("data/decks/eval/azorius-control.txt");
        let by_key = Seat {
            key: Some("eval/red-burn".into()),
            cards: None,
        };
        assert_eq!(book.identify(&by_key, 0.8, &mut cache).how, "key");
        // Another corpus's key for the same cards (a collector's `house-002`).
        let by_cards = Seat {
            key: Some("house-002".into()),
            cards: Some(azorius.clone()),
        };
        let found = book.identify(&by_cards, 0.8, &mut cache);
        assert_eq!(found.id.as_deref(), Some("eval/azorius-control"));
        assert_eq!(found.how, "cards");
        // One copy fewer of one card is still the same deck's lineage.
        let mut cut = azorius.clone();
        cut.remove(0);
        cut.push(u32::MAX - 1);
        cut.sort_unstable();
        let near = book.identify(
            &Seat {
                key: None,
                cards: Some(cut),
            },
            0.8,
            &mut cache,
        );
        assert_eq!(near.id.as_deref(), Some("eval/azorius-control"));
        assert!(near.how.starts_with("near"), "{}", near.how);
        let stranger = Seat {
            key: Some("gen-con-001".into()),
            cards: Some(vec![1, 2, 3]),
        };
        assert_eq!(book.identify(&stranger, 0.8, &mut cache).id, None);
    }

    /// A pool's manifest is the original's entries, unchanged, and no other.
    #[test]
    fn a_pool_is_a_copy_of_its_entries() {
        let manifest = json!({"version": 4, "decks": [
            {"id": "a", "split": "test", "tier": "x"},
            {"id": "b", "split": "train"},
            {"id": "c", "split": "val"},
        ]});
        let copy = manifest_of(&manifest, &BTreeSet::from(["a", "c"]));
        assert_eq!(copy["version"], 4);
        assert_eq!(
            copy["decks"],
            json!([manifest["decks"][0], manifest["decks"][2]])
        );
        assert_eq!(pool("test", false), Some("clean"));
        assert_eq!(pool("val", false), Some("clean"));
        assert_eq!(pool("test", true), Some("held"));
        assert_eq!(pool("train", false), Some("unseen-train"));
    }

    #[test]
    fn a_row_names_its_seats_by_key_strings_or_key_objects() {
        assert_eq!(
            row_keys(&json!({"decks": ["a/b", "c/d"]})),
            [Some("a/b".to_owned()), Some("c/d".to_owned())]
        );
        assert_eq!(
            row_keys(&json!({"decks": [{"key": "house-002", "name": "Victory"}, 3]})),
            [Some("house-002".to_owned()), None]
        );
        assert!(row_keys(&json!({"game": "x"})).is_empty());
    }

    /// A run of two games whose rows name no deck: the headers do, and the
    /// run's own list names their cards.
    #[test]
    fn a_run_is_read_from_its_record_headers() {
        use std::io::Write as _;
        let book = book();
        let dir = std::env::temp_dir().join(format!("seated-decks-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("records")).unwrap();
        let azorius = cards("data/decks/eval/azorius-control.txt");
        let burn = cards("data/decks/eval/red-burn.txt");
        let seat = |cards: &Cards| {
            json!({"deck": cards.iter().map(|c| json!({"card": c})).collect::<Vec<_>>(),
                "commanders": []})
        };
        let mut shard = Vec::new();
        let mut rows = Vec::new();
        for (game, seats) in [[&azorius, &burn], [&burn, &burn]].iter().enumerate() {
            let header =
                json!({"kind": "header", "preset": {"seats": [seat(seats[0]), seat(seats[1])]}});
            let mut member = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
            writeln!(member, "{header}").unwrap();
            let member = member.finish().unwrap();
            let row = json!({"game": game, "shard": "w00.jsonl.gz", "offset": shard.len(),
                "len": member.len()});
            writeln!(rows, "{row}").unwrap();
            shard.extend(member);
        }
        std::fs::write(dir.join("records/w00.jsonl.gz"), shard).unwrap();
        std::fs::write(dir.join("games.jsonl"), rows).unwrap();
        std::fs::write(
            dir.join("run.json"),
            serde_json::to_vec(&json!({"decks": [{"key": "eval/azorius-control",
                "cards": azorius}]}))
            .unwrap(),
        )
        .unwrap();
        let (seated, _) = read_run(&dir, &book, 0.8).unwrap();
        assert_eq!(seated.games, 2);
        assert_eq!(
            seated.decks["eval/azorius-control"],
            ("key+cards".to_owned(), 1)
        );
        assert_eq!(seated.decks["eval/red-burn"], ("cards".to_owned(), 3));
        assert_eq!(seated.games_with["test"], 1);
        assert_eq!(seated.games_with["train"], 2);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
