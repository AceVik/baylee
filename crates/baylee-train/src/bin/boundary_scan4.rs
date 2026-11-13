//! Which recorded games cross the activator boundary (eb40dc593).
//!
//! A record written before that commit carries other engine hashes after a
//! loyalty ability, a permanent turned face up, a door unlocked or a
//! prepared cast, and a later `convert4` refuses it: it is converted by the
//! runtime that wrote it (`docs/engine-internals.md` §"Forking a
//! simulation"). This reads runs, read only, and writes one JSON line per
//! run and a last line totalling them: its games, the games that cross
//! (`crossing`, read from their actions, not played:
//! `convert4::activator_boundary::crossings`), the games that open with this
//! build's hash (`opening_here`), the games a `convert4` of this build
//! refuses by the boundary (`refused_here`: both), and how often each kind
//! of action occurs.
//!
//! ```text
//! boundary_scan4 --root ~/baylee-data/v6 --out scan.jsonl
//! boundary_scan4 --dataset DATASET/meta.json --out dataset.jsonl
//! ```

// Looks cards up, so this program holds the pool (`baylee-cards-pool`).
use baylee_cards_pool as _;
use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::{BufRead as _, BufReader, Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};

use anyhow::{Context as _, ensure};
use baylee_train::convert4::activator_boundary::{Crossings, crossings, opens_as_this_build};
use clap::Parser;
use flate2::read::GzDecoder;
use serde_json::{Value, json};

#[derive(Parser)]
struct Args {
    /// Directories searched for runs: every `games.jsonl` with a `records/`
    /// beside it. Repeat it for several.
    #[arg(long)]
    root: Vec<PathBuf>,
    /// Run directories scanned as they are.
    #[arg(long)]
    run: Vec<PathBuf>,
    /// A converted LLM-BC dataset's `meta.json`: its `source_runs` are
    /// scanned, and of their games only those it converted
    /// (`llm_bc_sources`, by record digest).
    #[arg(long)]
    dataset: Option<PathBuf>,
    /// Destination JSON Lines, written only if it does not already exist.
    #[arg(long)]
    out: PathBuf,
}

/// Every run directory under `root`, in path order.
fn runs(root: &Path, found: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    let mut entries: Vec<PathBuf> = fs::read_dir(root)
        .with_context(|| format!("reading {}", root.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect();
    entries.sort();
    if root.join("games.jsonl").is_file() && root.join("records").is_dir() {
        found.push(root.to_path_buf());
    }
    for path in entries {
        // A symbolic link is not followed: a run is counted where it lives.
        if path.is_dir() && !path.is_symlink() {
            runs(&path, found)?;
        }
    }
    Ok(())
}

/// A converted LLM-BC dataset's source runs, added to `found`, and the
/// digests of the records it converted.
fn dataset(meta: &Path, found: &mut Vec<PathBuf>) -> anyhow::Result<BTreeSet<String>> {
    let meta: Value = serde_json::from_slice(
        &fs::read(meta).with_context(|| format!("reading {}", meta.display()))?,
    )?;
    let runs = meta["source_runs"].as_array().context("no source_runs")?;
    found.extend(runs.iter().filter_map(Value::as_str).map(PathBuf::from));
    let sources = meta["llm_bc_sources"]
        .as_array()
        .context("no llm_bc_sources")?;
    sources
        .iter()
        .map(|source| {
            source["record_sha256"]
                .as_str()
                .map(str::to_owned)
                .context("a source without record_sha256")
        })
        .collect()
}

fn sha256(bytes: &[u8]) -> String {
    baylee_train::convert4::llm_bc::sha(bytes)
}

/// One gzip member of a record shard.
fn member(path: &Path, offset: u64, len: u64) -> anyhow::Result<Vec<u8>> {
    let mut file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    file.seek(SeekFrom::Start(offset))?;
    let mut out = Vec::new();
    GzDecoder::new(file.take(len)).read_to_end(&mut out)?;
    Ok(out)
}

/// What a set of games holds.
#[derive(Default)]
struct Tally {
    games: u64,
    crossing: u64,
    /// Games that open with this build's hash (`opens_as_this_build`).
    opening_here: u64,
    /// Games that cross and open with this build's hash: the ones a
    /// `convert4` built from here refuses by the boundary.
    refused_here: u64,
    unreadable: u64,
    actions: Crossings,
}

impl Tally {
    fn add(&mut self, other: &Self) {
        self.games += other.games;
        self.crossing += other.crossing;
        self.opening_here += other.opening_here;
        self.refused_here += other.refused_here;
        self.unreadable += other.unreadable;
        self.actions.loyalty += other.actions.loyalty;
        self.actions.face_up += other.actions.face_up;
        self.actions.door += other.actions.door;
        self.actions.prepared += other.actions.prepared;
    }

    fn json(&self, name: &str, value: &Value) -> Value {
        json!({
            name: value,
            "games": self.games,
            "crossing": self.crossing,
            "opening_here": self.opening_here,
            "refused_here": self.refused_here,
            "unreadable": self.unreadable,
            "loyalty": self.actions.loyalty,
            "face_up": self.actions.face_up,
            "door": self.actions.door,
            "prepared": self.actions.prepared,
        })
    }
}

/// Every game of one run, or of those whose record digest `only` holds.
fn scan(run: &Path, only: Option<&BTreeSet<String>>) -> anyhow::Result<Tally> {
    let index = BufReader::new(File::open(run.join("games.jsonl"))?);
    let mut tally = Tally::default();
    for line in index.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let game: Value = serde_json::from_str(&line)?;
        let read = || -> anyhow::Result<Option<(Crossings, bool)>> {
            let shard = game["shard"].as_str().context("no shard")?;
            let offset = game["offset"].as_u64().context("no offset")?;
            let len = game["len"].as_u64().context("no len")?;
            let record = member(&run.join("records").join(shard), offset, len)?;
            if only.is_some_and(|only| !only.contains(&sha256(&record))) {
                return Ok(None);
            }
            let found = crossings(&record).map_err(anyhow::Error::msg)?;
            Ok(Some((found, opens_as_this_build(&record))))
        };
        let (found, opens_here) = match read() {
            Ok(Some(found)) => found,
            Ok(None) => continue,
            Err(_) => {
                tally.games += 1;
                tally.unreadable += 1;
                continue;
            }
        };
        tally.games += 1;
        tally.crossing += u64::from(found.any());
        tally.opening_here += u64::from(opens_here);
        tally.refused_here += u64::from(opens_here && found.any());
        tally.add(&Tally {
            actions: found,
            ..Tally::default()
        });
    }
    Ok(tally)
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    ensure!(
        !args.root.is_empty() || !args.run.is_empty() || args.dataset.is_some(),
        "name a --root, a --run or a --dataset"
    );
    ensure!(!args.out.exists(), "{} exists", args.out.display());
    let mut found = args.run.clone();
    for root in &args.root {
        runs(root, &mut found)?;
    }
    let only = args
        .dataset
        .as_deref()
        .map(|meta| dataset(meta, &mut found))
        .transpose()?;
    let mut out = File::create_new(&args.out)?;
    let mut total = Tally::default();
    for run in &found {
        let line = match scan(run, only.as_ref()) {
            Ok(tally) => {
                total.add(&tally);
                tally.json("run", &json!(run))
            }
            Err(e) => json!({"run": run, "error": e.to_string()}),
        };
        writeln!(out, "{line}")?;
        eprintln!("{line}");
    }
    let line = total.json("runs", &json!(found.len()));
    writeln!(out, "{line}")?;
    eprintln!("{line}");
    Ok(())
}
