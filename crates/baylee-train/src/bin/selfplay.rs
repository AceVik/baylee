//! Plays and records house-AI games for the trainer, on every core.
//!
//! ```text
//! cargo run --profile selfplay -p baylee-train --bin selfplay -- \
//!     --decks allytifact,victory --games 10000 --out ~/baylee-data/runs/r001
//! ```
//!
//! Writes into `--out`:
//!
//! - `run.json`: what was played and with what, stamped with the build and
//!   the working-card set's hash ([`baylee_train::working`]);
//! - `games.jsonl`: one line per game (seed, decks, profiles, outcome, where
//!   its record is);
//! - `records/w<worker>-<shard>.jsonl.gz`: the records, one gzip member per
//!   game, found by the `offset` and `len` its line in `games.jsonl` gives.
//!
//! A game is played by one thread from start to end, and which game gets
//! which seed, decks and profiles depends only on its number, so a run is the
//! same games however many threads play it.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{BufWriter, Write as _};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context as _, bail};
use baylee_core::ids::CardIndex;
use baylee_core::preset::AIProfile;
use baylee_train::housedeck::HouseDeck;
use baylee_train::selfplay::{Caps, Outcome, play, table};
use baylee_train::working::{Working, repo_root};
use clap::Parser;
use flate2::Compression;
use flate2::write::GzEncoder;
use serde_json::{Value, json};

#[derive(Parser, Debug)]
#[command(about = "Plays and records house-AI games for the trainer")]
struct Args {
    /// House decks (`data/decks/<key>.txt`), comma-separated. Every ordered
    /// pair of two different decks is a matchup; games cycle through them.
    #[arg(long, value_delimiter = ',', default_value = "allytifact,victory")]
    decks: Vec<String>,
    /// House profiles the chairs are drawn from, per seat and game.
    #[arg(
        long,
        value_delimiter = ',',
        default_value = "novice,casual,steady,sharp,expert"
    )]
    profiles: Vec<String>,
    /// Games to play.
    #[arg(long, default_value_t = 1000)]
    games: u64,
    /// The first game's seed; game `i` is dealt with `first_seed + i`.
    #[arg(long, default_value_t = 1)]
    first_seed: u64,
    /// Threads; 0 = one per core.
    #[arg(long, default_value_t = 0)]
    threads: usize,
    /// Answers a game may take before it is stopped.
    #[arg(long, default_value_t = 20_000)]
    max_answers: u64,
    /// Seconds a game may take before it is stopped.
    #[arg(long, default_value_t = 60)]
    max_secs: u64,
    /// Games per record shard.
    #[arg(long, default_value_t = 1000)]
    shard_games: u64,
    /// Play decks with cards that do not work (the games are then no
    /// training data, and `run.json` says so).
    #[arg(long)]
    allow_failing: bool,
    /// Play each deck cut to its working cards
    /// (`HouseDeck::working_only`): what does not work becomes the deck's
    /// own basic lands. The games are training data.
    #[arg(long, conflicts_with = "allow_failing")]
    working_only: bool,
    /// Where the run is written; must not exist yet.
    #[arg(long)]
    out: PathBuf,
}

/// Which game gets which chairs: a pure function of its number.
struct Schedule {
    first_seed: u64,
    pairs: Vec<(usize, usize)>,
    profiles: Vec<(&'static str, AIProfile)>,
}

/// One game's assignment.
struct Assigned {
    seed: u64,
    decks: (usize, usize),
    profiles: [usize; 2],
}

/// `SplitMix64`: spreads consecutive seeds over the profile pairs.
const fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

impl Schedule {
    fn game(&self, i: u64) -> Assigned {
        let seed = self.first_seed + i;
        let n = self.profiles.len() as u64;
        let pick = mix(seed);
        Assigned {
            seed,
            decks: self.pairs[(i % self.pairs.len() as u64) as usize],
            profiles: [(pick % n) as usize, ((pick >> 32) % n) as usize],
        }
    }
}

/// Per worker, the game it is playing and since when.
type Watch = Arc<Mutex<Vec<Option<(u64, Instant)>>>>;

/// A finished game, as a worker hands it to the writer.
struct Done {
    i: u64,
    line: Value,
    outcome: Outcome,
}

/// A shard of records a worker appends to.
struct Shard {
    path: PathBuf,
    file: BufWriter<File>,
    written: u64,
    games: u64,
}

impl Shard {
    fn open(dir: &Path, worker: usize, number: u64) -> anyhow::Result<Self> {
        let name = format!("w{worker:02}-{number:04}.jsonl.gz");
        let path = dir.join(&name);
        Ok(Self {
            file: BufWriter::new(File::create(&path)?),
            path,
            written: 0,
            games: 0,
        })
    }

    /// Appends one record as its own gzip member; `(offset, len)`.
    fn append(&mut self, record: &[u8]) -> anyhow::Result<(u64, u64)> {
        let mut gz = GzEncoder::new(Vec::with_capacity(record.len() / 6), Compression::new(6));
        gz.write_all(record)?;
        let bytes = gz.finish()?;
        let at = self.written;
        self.file.write_all(&bytes)?;
        self.written += bytes.len() as u64;
        self.games += 1;
        Ok((at, bytes.len() as u64))
    }
}

/// `n / d` for the progress line, with an empty `d` counted as one. Game
/// counts stay far below 2^52, past which an `f64` skips integers.
#[allow(clippy::cast_precision_loss)]
fn ratio(n: u64, d: u64) -> f64 {
    n as f64 / d.max(1) as f64
}

fn unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

#[allow(clippy::too_many_lines)] // setup, workers and the writer read best in one place
fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    if args.out.exists() {
        bail!("{} exists; a run is never written over", args.out.display());
    }
    let working = Working::scan(&repo_root()).context("reading the card and combo tests")?;
    let mut decks: Vec<HouseDeck> = args
        .decks
        .iter()
        .map(|key| HouseDeck::named(key))
        .collect::<anyhow::Result<_>>()?;
    if decks.len() < 2 {
        bail!("a duel needs two decks");
    }
    let mut replaced = Vec::new();
    if args.working_only {
        for deck in &mut decks {
            let (cut, removed) = deck.working_only(&working)?;
            eprintln!(
                "[selfplay] {}: {} cards ({} copies) that do not work replaced by the deck's basics",
                deck.key,
                removed.len(),
                removed.iter().map(|(_, n)| n).sum::<u32>()
            );
            replaced.push(
                removed
                    .iter()
                    .map(|(c, n)| json!({"card": c.get(), "copies": n}))
                    .collect::<Vec<_>>(),
            );
            *deck = cut;
        }
    }
    let mut deck_json = Vec::new();
    let mut failing_any = false;
    for (at, deck) in decks.iter().enumerate() {
        let failing = deck.failing(&working);
        for (card, why) in &failing {
            let name = baylee_cards::by_index(*card).map_or("?", |d| d.name());
            eprintln!("[selfplay] {}: {name} ({card}) {why}", deck.key);
        }
        failing_any |= !failing.is_empty();
        deck_json.push(json!({
            "key": deck.key,
            "name": deck.deck.name,
            "cards": deck.cards().map(CardIndex::get).collect::<Vec<_>>(),
            "failing": failing.iter().map(|(c, why)| json!({"card": c.get(), "why": why.to_string()})).collect::<Vec<_>>(),
            "replaced": replaced.get(at),
        }));
    }
    if failing_any && !args.allow_failing {
        bail!("a deck holds cards that do not work (above); --allow-failing plays it anyway");
    }
    let profiles: Vec<(&'static str, AIProfile)> = args
        .profiles
        .iter()
        .map(|key| {
            AIProfile::NAMED
                .iter()
                .find(|(name, _)| name == key)
                .copied()
                .with_context(|| format!("unknown profile {key}"))
        })
        .collect::<anyhow::Result<_>>()?;
    let pairs: Vec<(usize, usize)> = (0..decks.len())
        .flat_map(|a| (0..decks.len()).map(move |b| (a, b)))
        .filter(|(a, b)| a != b)
        .collect();
    let threads = match args.threads {
        0 => std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get),
        n => n,
    };
    let caps = Caps {
        answers: args.max_answers,
        wall: Duration::from_secs(args.max_secs),
    };
    let run = args
        .out
        .file_name()
        .and_then(|n| n.to_str())
        .context("--out names a directory")?
        .to_owned();
    let records = args.out.join("records");
    fs::create_dir_all(&records)?;

    let mut manifest = json!({
        "run": run,
        "record_version": baylee_gamehost::record::RECORD_VERSION,
        "build": baylee_build::short(),
        "commit": baylee_build::COMMIT,
        "dirty": baylee_build::DIRTY,
        "working": {
            "rule": "Coverage::Implemented and named by card_index(\"…\") in card_tests/ or combo_tests/",
            "pool": working.pool,
            "implemented": working.implemented,
            "tested": working.tested.len(),
            "works": working.cards.len(),
            "hash": working.hash(),
            "unknown_test_ids": working.unknown,
        },
        "training_data": !failing_any,
        "decks": deck_json,
        "profiles": profiles.iter().map(|(n, p)| json!({"name": n, "profile": p})).collect::<Vec<_>>(),
        "games": args.games,
        "first_seed": args.first_seed,
        "caps": {"answers": caps.answers, "secs": args.max_secs},
        "threads": threads,
        "started": unix_secs(),
    });
    fs::write(
        args.out.join("run.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    eprintln!(
        "[selfplay] {run}: {} games on {threads} threads · build {} · {} of {} pool cards work (hash {})",
        args.games,
        baylee_build::short(),
        working.cards.len(),
        working.pool,
        &working.hash()[..12],
    );

    let schedule = Arc::new(Schedule {
        first_seed: args.first_seed,
        pairs,
        profiles,
    });
    let decks = Arc::new(decks);
    let next = Arc::new(AtomicU64::new(0));
    // What each worker is playing and since when, for the stuck-game watch.
    let playing: Watch = Arc::new(Mutex::new(vec![None; threads]));
    let (tx, rx) = mpsc::channel::<anyhow::Result<Done>>();
    for worker in 0..threads {
        let (schedule, decks, next, playing, tx) = (
            schedule.clone(),
            decks.clone(),
            next.clone(),
            playing.clone(),
            tx.clone(),
        );
        let (records, run, games, shard_games) =
            (records.clone(), run.clone(), args.games, args.shard_games);
        std::thread::spawn(move || {
            let mut shard: Option<Shard> = None;
            let mut shards = 0;
            loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                if i >= games {
                    break;
                }
                playing.lock().expect("the watch lock")[worker] = Some((i, Instant::now()));
                let result = (|| -> anyhow::Result<Done> {
                    let game = schedule.game(i);
                    let (a, b) = (&decks[game.decks.0], &decks[game.decks.1]);
                    let chairs = game.profiles.map(|p| schedule.profiles[p]);
                    let preset = table(game.seed, a, b, chairs.map(|(_, p)| p));
                    let id = format!("{run}-{i:07}");
                    let played = play(&preset, &id, caps);
                    if shard.as_ref().is_none_or(|s| s.games >= shard_games) {
                        shard = Some(Shard::open(&records, worker, shards)?);
                        shards += 1;
                    }
                    let shard = shard.as_mut().expect("opened above");
                    let (offset, len) = shard.append(&played.record)?;
                    let line = json!({
                        "game": id,
                        "i": i,
                        "seed": game.seed,
                        "decks": [a.key, b.key],
                        "profiles": chairs.map(|(n, _)| n),
                        "outcome": played.outcome,
                        "answers": played.answers,
                        "turn": played.turn,
                        "ms": played.elapsed.as_millis() as u64,
                        "shard": shard.path.file_name().and_then(|n| n.to_str()),
                        "offset": offset,
                        "len": len,
                    });
                    Ok(Done {
                        i,
                        line,
                        outcome: played.outcome,
                    })
                })();
                playing.lock().expect("the watch lock")[worker] = None;
                if tx.send(result).is_err() {
                    break;
                }
            }
            if let Some(mut shard) = shard {
                let _ = shard.file.flush();
            }
        });
    }
    drop(tx);

    let mut index = BufWriter::new(File::create(args.out.join("games.jsonl"))?);
    let mut kinds: BTreeMap<&'static str, u64> = BTreeMap::new();
    let mut seat0_wins = 0_u64;
    // Wins of each deck as seat 0 and as seat 1, over decided games.
    let mut deck_wins: BTreeMap<String, (u64, u64)> = BTreeMap::new();
    let mut received = 0_u64;
    let started = Instant::now();
    let mut last_report = Instant::now();
    let mut stuck: Vec<u64> = Vec::new();
    let stuck_after = caps.wall.saturating_mul(10).max(Duration::from_secs(60));
    while received + (stuck.len() as u64) < args.games {
        match rx.recv_timeout(Duration::from_secs(1)) {
            Ok(done) => {
                let done = done?;
                received += 1;
                let kind = match &done.outcome {
                    Outcome::Won { seat, .. } => {
                        seat0_wins += u64::from(*seat == 0);
                        "won"
                    }
                    Outcome::Draw { .. } => "draw",
                    Outcome::AnswerCap => "answer_cap",
                    Outcome::TimeCap => "time_cap",
                    Outcome::Panicked { .. } => "panicked",
                };
                if let Outcome::Won { seat, .. } = &done.outcome {
                    let decks_played = done.line["decks"].as_array().cloned().unwrap_or_default();
                    for (s, deck) in decks_played.iter().enumerate() {
                        let entry = deck_wins
                            .entry(deck.as_str().unwrap_or("?").to_owned())
                            .or_default();
                        entry.1 += 1;
                        if s == usize::from(*seat) {
                            entry.0 += 1;
                        }
                    }
                }
                *kinds.entry(kind).or_default() += 1;
                serde_json::to_writer(&mut index, &done.line)?;
                index.write_all(b"\n")?;
                if !done.outcome.finished() {
                    eprintln!(
                        "[selfplay] game {} seed {}: {}",
                        done.i, done.line["seed"], done.line["outcome"]
                    );
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        for (i, since) in playing.lock().expect("the watch lock").iter().flatten() {
            if since.elapsed() > stuck_after && !stuck.contains(i) {
                let seed = schedule.game(*i).seed;
                eprintln!(
                    "[selfplay] game {i} (seed {seed}) has been inside one answer past every cap for {}s; counted as stuck",
                    since.elapsed().as_secs()
                );
                stuck.push(*i);
            }
        }
        if last_report.elapsed() >= Duration::from_secs(5)
            || received + (stuck.len() as u64) == args.games
        {
            last_report = Instant::now();
            let secs = started.elapsed().as_secs_f64();
            let decided = kinds.get("won").copied().unwrap_or(0);
            let pct = |n: u64| 100.0 * ratio(n, received);
            eprintln!(
                "[selfplay] {received}/{} games · {:.1} games/s · seat 0 wins {:.1}% of decided · draw {:.1}% · answer cap {:.1}% · time cap {:.1}% · panics {} · stuck {}",
                args.games,
                ratio(received, 1) / secs.max(1e-9),
                100.0 * ratio(seat0_wins, decided),
                pct(kinds.get("draw").copied().unwrap_or(0)),
                pct(kinds.get("answer_cap").copied().unwrap_or(0)),
                pct(kinds.get("time_cap").copied().unwrap_or(0)),
                kinds.get("panicked").copied().unwrap_or(0),
                stuck.len(),
            );
        }
    }
    index.flush()?;
    for i in &stuck {
        let game = schedule.game(*i);
        eprintln!("[selfplay] stuck: game {i}, seed {}", game.seed);
    }
    let decks_report: BTreeMap<&String, Value> = deck_wins
        .iter()
        .map(|(deck, (won, played))| (deck, json!({"won": won, "decided": played})))
        .collect();
    manifest["finished"] = json!(unix_secs());
    manifest["seconds"] = json!(started.elapsed().as_secs_f64());
    manifest["totals"] = json!({
        "played": received,
        "outcomes": kinds,
        "seat0_wins": seat0_wins,
        "decks": decks_report,
        "stuck": stuck.iter().map(|i| json!({"i": i, "seed": schedule.game(*i).seed})).collect::<Vec<_>>(),
    });
    fs::write(
        args.out.join("run.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    eprintln!(
        "[selfplay] done: {received} games in {:.1}s → {}",
        started.elapsed().as_secs_f64(),
        args.out.display()
    );
    // A thread stuck inside one answer never returns; the process ends
    // without waiting for it, its game already counted.
    std::process::exit(0);
}
