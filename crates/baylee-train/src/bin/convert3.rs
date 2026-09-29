//! Turns self-play runs into a v3 dataset (`baylee_train::features3`).
//!
//! ```text
//! cargo run --profile selfplay -p baylee-train --bin convert3 -- \
//!     --runs ~/baylee-data/runs/r010 --out ~/baylee-data/datasets/d010
//! ```
//!
//! As `convert` does for v2: only games that ended by their rules, a record
//! that no longer replays refused whole, one shard of flat little-endian
//! column files per worker:
//!
//! - `ent_card.i32`, `ent_feat.i16` (entities × `ENT_COLS`), `ent_off.i64`
//!   (samples + 1);
//! - `seat.i16` (samples × `MAX_SEATS` × `SEAT_COLS`, rows past the table's
//!   seats zero), `glob.i16` (samples × `GLOB_COLS`), `meta.i32` (samples ×
//!   `META_COLS`);
//! - `deck_card.i32`, `deck_feat.i16` (× `DECK_COLS`), `deck_off.i64`: the
//!   deciding seat's own list;
//! - `opt.i16` (options × 3), `opt_off.i64`;
//! - the hidden half: `omni_card.i32`, `omni_tag.i16`, `omni_pos.i16`,
//!   `omni_off.i64`, and every seat's list, `omni_deck_card.i32`,
//!   `omni_deck_feat.i16` (× 3: seat, in list, in library),
//!   `omni_deck_off.i64`.
//!
//! The dataset's root holds the card table once: `card_ids.i32` and
//! `card_feats.f32` (cards × `cardwalk::WIDTH`).

use std::fs::{self, File};
use std::io::{BufRead as _, BufReader, BufWriter, Read as _, Seek as _, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use anyhow::{Context as _, bail};
use baylee_train::convert3::{Converted, Keep, Refused, convert};
use baylee_train::features::{
    ID_SPACE, LEDGER_ROWS, MAX_ENTITIES, PENDING_KINDS, TOKEN_BASE, UNKNOWN_ID,
};
use baylee_train::features3::{
    DECK_COLS, ENT_COLS, GLOB_COLS, MAX_DECK, MAX_SEATS, SEAT_COLS, VERSION, card_table,
};
use clap::Parser;
use flate2::read::GzDecoder;
use serde_json::{Value, json};

/// Columns of `meta.i32`.
const META_COLS: [&str; 18] = [
    "game",
    "seat",
    "n",
    "turn",
    "result",
    "final_turn",
    "pending_kind",
    "n_options",
    "entities_dropped",
    "offered_dropped",
    "step",
    "chosen",
    "n_opts",
    "profile",
    "seats",
    "winners_rel",
    "alive_rel",
    "team_rel",
];

/// The house profiles, as the `profile` column numbers them.
const PROFILES: [&str; 5] = ["novice", "casual", "steady", "sharp", "expert"];

/// Outcomes a game is converted with: it ended by its rules.
const FINISHED: [&str; 3] = ["won", "team_won", "draw"];

#[derive(Parser, Debug)]
#[command(about = "Turns self-play runs into a v3 dataset the trainer reads")]
struct Args {
    /// Run directories, comma-separated.
    #[arg(long, value_delimiter = ',', required = true)]
    runs: Vec<PathBuf>,
    /// Where the dataset is written; must not exist yet.
    #[arg(long)]
    out: PathBuf,
    /// Threads; 0 = one per core.
    #[arg(long, default_value_t = 0)]
    threads: usize,
    /// Keep questions with a single answer (forced passes).
    #[arg(long)]
    keep_forced: bool,
    /// Keep every n-th decision of a game, after the forced ones are dropped.
    #[arg(long, default_value_t = 1)]
    every: u32,
    /// Convert a run whose `run.json` says it is no training data.
    #[arg(long)]
    allow_untrusted: bool,
}

/// A game to convert.
struct Game {
    id: u32,
    run: usize,
    line: Value,
}

/// What a worker reports per game.
struct Report {
    id: u32,
    samples: usize,
    refused: Option<String>,
    unmatched: std::collections::BTreeMap<(i16, &'static str), u64>,
    offered_dropped: u64,
}

/// The files of a shard, in order.
const FILES: [&str; 18] = [
    "ent_card.i32",
    "ent_feat.i16",
    "ent_off.i64",
    "seat.i16",
    "glob.i16",
    "meta.i32",
    "deck_card.i32",
    "deck_feat.i16",
    "deck_off.i64",
    "opt.i16",
    "opt_off.i64",
    "omni_card.i32",
    "omni_tag.i16",
    "omni_pos.i16",
    "omni_off.i64",
    "omni_deck_card.i32",
    "omni_deck_feat.i16",
    "omni_deck_off.i64",
];

/// One worker's column files, by [`FILES`] position, and the running totals
/// the offset files count.
struct Shard {
    dir: PathBuf,
    files: Vec<BufWriter<File>>,
    samples: u64,
    entities: u64,
    decks: u64,
    opts: u64,
    omni: u64,
    omni_decks: u64,
}

/// A little-endian value's bytes.
trait Le {
    fn le(&self) -> Vec<u8>;
}
impl Le for i16 {
    fn le(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
}
impl Le for i32 {
    fn le(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
}
impl Le for i64 {
    fn le(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
}

impl Shard {
    fn create(dir: PathBuf) -> anyhow::Result<Self> {
        fs::create_dir_all(&dir)?;
        let files = FILES
            .iter()
            .map(|name| {
                Ok(BufWriter::with_capacity(
                    1 << 20,
                    File::create(dir.join(name))?,
                ))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        let mut shard = Self {
            dir,
            files,
            samples: 0,
            entities: 0,
            decks: 0,
            opts: 0,
            omni: 0,
            omni_decks: 0,
        };
        for f in [
            "ent_off.i64",
            "deck_off.i64",
            "opt_off.i64",
            "omni_off.i64",
            "omni_deck_off.i64",
        ] {
            shard.put(f, &0_i64)?;
        }
        Ok(shard)
    }

    fn put(&mut self, file: &str, v: &dyn Le) -> std::io::Result<()> {
        let at = FILES.iter().position(|f| *f == file).expect("a shard file");
        self.files[at].write_all(&v.le())
    }

    fn write(&mut self, game: u32, profiles: &[i64], converted: &Converted) -> anyhow::Result<()> {
        for s in &converted.samples {
            let a = &s.actor;
            for card in &a.cards {
                self.put("ent_card.i32", card)?;
            }
            for row in &a.rows {
                for v in row {
                    self.put("ent_feat.i16", v)?;
                }
            }
            self.entities += a.cards.len() as u64;
            self.put("ent_off.i64", &(self.entities as i64))?;
            for r in 0..MAX_SEATS {
                let row = a.seats.get(r).copied().unwrap_or([0; SEAT_COLS.len()]);
                for v in row {
                    self.put("seat.i16", &v)?;
                }
            }
            for v in &a.globals {
                self.put("glob.i16", v)?;
            }
            for card in &a.deck_cards {
                self.put("deck_card.i32", card)?;
            }
            for row in &a.deck_rows {
                for v in row {
                    self.put("deck_feat.i16", v)?;
                }
            }
            self.decks += a.deck_cards.len() as u64;
            self.put("deck_off.i64", &(self.decks as i64))?;
            for o in &s.opts {
                for v in [o.head, o.a, o.b] {
                    self.put("opt.i16", &v)?;
                }
            }
            self.opts += s.opts.len() as u64;
            self.put("opt_off.i64", &(self.opts as i64))?;
            let result = if converted.winners.is_empty() {
                1
            } else if converted.winners.contains(&s.seat) {
                2
            } else {
                0
            };
            let meta: [i64; META_COLS.len()] = [
                i64::from(game),
                i64::from(s.seat),
                s.n as i64,
                i64::from(s.turn),
                result,
                i64::from(converted.final_turn),
                i64::from(a.kind),
                i64::from(a.options),
                a.dropped as i64,
                a.offered_dropped as i64,
                i64::from(s.step),
                i64::from(s.chosen),
                s.opts.len() as i64,
                profiles.get(usize::from(s.seat)).copied().unwrap_or(-1),
                converted.teams.len() as i64,
                converted.relative_mask(s.seat, &converted.winners),
                converted.relative_mask(s.seat, &converted.alive),
                converted.team_mask(s.seat),
            ];
            for v in meta {
                self.put("meta.i32", &i32::try_from(v).unwrap_or(i32::MAX))?;
            }
            let o = &s.omni;
            for ((card, tag), pos) in o.cards.iter().zip(&o.tags).zip(&o.positions) {
                self.put("omni_card.i32", card)?;
                self.put("omni_tag.i16", tag)?;
                self.put("omni_pos.i16", pos)?;
            }
            self.omni += o.cards.len() as u64;
            self.put("omni_off.i64", &(self.omni as i64))?;
            for card in &o.deck_cards {
                self.put("omni_deck_card.i32", card)?;
            }
            for row in &o.deck_rows {
                for v in row {
                    self.put("omni_deck_feat.i16", v)?;
                }
            }
            self.omni_decks += o.deck_cards.len() as u64;
            self.put("omni_deck_off.i64", &(self.omni_decks as i64))?;
            self.samples += 1;
        }
        Ok(())
    }

    fn finish(mut self) -> anyhow::Result<Value> {
        for f in &mut self.files {
            f.flush()?;
        }
        Ok(json!({
            "dir": self.dir.file_name().and_then(|n| n.to_str()),
            "samples": self.samples,
            "entities": self.entities,
            "decks": self.decks,
            "opts": self.opts,
            "omni": self.omni,
            "omni_decks": self.omni_decks,
        }))
    }
}

/// A game's record, from its run's shards.
fn read_record(run: &Path, line: &Value) -> anyhow::Result<Vec<u8>> {
    let shard = line["shard"].as_str().context("a shard")?;
    let offset = line["offset"].as_u64().context("an offset")?;
    let len = usize::try_from(line["len"].as_u64().context("a length")?)?;
    let mut file = File::open(run.join("records").join(shard))?;
    file.seek(SeekFrom::Start(offset))?;
    let mut member = vec![0; len];
    file.read_exact(&mut member)?;
    let mut record = Vec::new();
    GzDecoder::new(&member[..]).read_to_end(&mut record)?;
    Ok(record)
}

/// Writes the card table at the dataset's root.
fn write_card_table(out: &Path) -> anyhow::Result<usize> {
    let (ids, values) = card_table();
    let mut f = BufWriter::new(File::create(out.join("card_ids.i32"))?);
    for id in &ids {
        f.write_all(&id.to_le_bytes())?;
    }
    f.flush()?;
    let mut f = BufWriter::new(File::create(out.join("card_feats.f32"))?);
    for v in &values {
        f.write_all(&v.to_le_bytes())?;
    }
    f.flush()?;
    Ok(ids.len())
}

#[allow(clippy::too_many_lines)] // setup, workers and the manifest read best in one place
fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    if args.out.exists() {
        bail!(
            "{} exists; a dataset is never written over",
            args.out.display()
        );
    }
    let mut sources = Vec::new();
    let mut games = Vec::new();
    let mut unfinished = 0_u64;
    for (r, run) in args.runs.iter().enumerate() {
        let manifest: Value = serde_json::from_slice(&fs::read(run.join("run.json"))?)
            .with_context(|| format!("{}/run.json", run.display()))?;
        if manifest["training_data"] != json!(true) && !args.allow_untrusted {
            bail!(
                "{} deals cards that do not work; --allow-untrusted converts it anyway",
                run.display()
            );
        }
        for row in BufReader::new(File::open(run.join("games.jsonl"))?).lines() {
            let line: Value = serde_json::from_str(&row?)?;
            let kind = line["outcome"]["kind"].as_str().unwrap_or("");
            if FINISHED.contains(&kind) {
                games.push(Game {
                    id: u32::try_from(games.len())?,
                    run: r,
                    line,
                });
            } else {
                unfinished += 1;
            }
        }
        sources.push(json!({
            "dir": run,
            "run": manifest["run"],
            "build": manifest["build"],
            "commit": manifest["commit"],
            "dirty": manifest["dirty"],
            "record_version": manifest["record_version"],
            "working": manifest["working"],
            "training_data": manifest["training_data"],
            "seats": manifest["seats"],
            "teams": manifest["teams"],
        }));
    }
    let threads = match args.threads {
        0 => std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get),
        n => n,
    };
    fs::create_dir_all(&args.out)?;
    let table_rows = write_card_table(&args.out)?;
    eprintln!(
        "[convert3] {} finished games from {} run(s) on {threads} threads ({unfinished} stopped games left out); card table {table_rows} rows",
        games.len(),
        args.runs.len()
    );

    let keep = Keep {
        forced: args.keep_forced,
        every: args.every,
    };
    let games = Arc::new(games);
    let runs = Arc::new(args.runs.clone());
    let next = Arc::new(AtomicUsize::new(0));
    let (tx, rx) = mpsc::channel::<Report>();
    let mut workers = Vec::new();
    for worker in 0..threads {
        let (games, runs, next, tx) = (games.clone(), runs.clone(), next.clone(), tx.clone());
        let dir = args.out.join(format!("shard-{worker:02}"));
        workers.push(std::thread::spawn(move || -> anyhow::Result<Value> {
            let mut shard = Shard::create(dir)?;
            loop {
                let at = next.fetch_add(1, Ordering::Relaxed);
                let Some(game) = games.get(at) else {
                    break;
                };
                let record = read_record(&runs[game.run], &game.line)?;
                let profiles: Vec<i64> = game.line["profiles"]
                    .as_array()
                    .map(|ps| {
                        ps.iter()
                            .map(|p| {
                                PROFILES
                                    .iter()
                                    .position(|n| Some(*n) == p.as_str())
                                    .map_or(-1, |i| i as i64)
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let report = match convert(&record, keep) {
                    Ok(converted) => {
                        shard.write(game.id, &profiles, &converted)?;
                        Report {
                            id: game.id,
                            samples: converted.samples.len(),
                            refused: None,
                            offered_dropped: converted
                                .samples
                                .iter()
                                .map(|s| s.actor.offered_dropped as u64)
                                .sum(),
                            unmatched: converted.unmatched,
                        }
                    }
                    Err(why) => Report {
                        id: game.id,
                        samples: 0,
                        refused: Some(match why {
                            Refused::Diverged(_) => "diverged".to_owned(),
                            other => format!("{other:?}"),
                        }),
                        unmatched: std::collections::BTreeMap::new(),
                        offered_dropped: 0,
                    },
                };
                if tx.send(report).is_err() {
                    break;
                }
            }
            shard.finish()
        }));
    }
    drop(tx);

    let started = Instant::now();
    let mut last = Instant::now();
    let mut reports: Vec<Option<Report>> = (0..games.len()).map(|_| None).collect();
    let mut done = 0_usize;
    let mut samples = 0_usize;
    let mut refused = std::collections::BTreeMap::<String, u64>::new();
    let mut unmatched = std::collections::BTreeMap::<String, u64>::new();
    let mut offered_dropped = 0_u64;
    loop {
        match rx.recv_timeout(Duration::from_secs(1)) {
            Ok(report) => {
                done += 1;
                samples += report.samples;
                offered_dropped += report.offered_dropped;
                if let Some(why) = &report.refused {
                    *refused.entry(why.clone()).or_default() += 1;
                }
                for ((kind, why), n) in &report.unmatched {
                    let kind = PENDING_KINDS.get(*kind as usize).copied().unwrap_or("?");
                    *unmatched.entry(format!("{kind}/{why}")).or_default() += n;
                }
                let id = report.id as usize;
                reports[id] = Some(report);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        if last.elapsed() >= Duration::from_secs(5) || done == games.len() {
            last = Instant::now();
            let secs = started.elapsed().as_secs_f64().max(1e-9);
            eprintln!(
                "[convert3] {done}/{} games · {samples} decisions · {:.0} games/s · refused {refused:?} · unmatched {unmatched:?} · offered dropped {offered_dropped}",
                games.len(),
                f64::from(u32::try_from(done).unwrap_or(u32::MAX)) / secs,
            );
            if done == games.len() {
                break;
            }
        }
    }
    let mut shards = Vec::new();
    for w in workers {
        shards.push(
            w.join()
                .map_err(|_| anyhow::anyhow!("a worker panicked"))??,
        );
    }

    let mut index = BufWriter::new(File::create(args.out.join("games.jsonl"))?);
    for (game, report) in games.iter().zip(&reports) {
        let line = &game.line;
        serde_json::to_writer(
            &mut index,
            &json!({
                "game": game.id,
                "run": game.run,
                "i": line["i"],
                "seed": line["seed"],
                "decks": line["decks"],
                "profiles": line["profiles"],
                "teams": line["teams"],
                "outcome": line["outcome"],
                "samples": report.as_ref().map_or(0, |r| r.samples),
                "refused": report.as_ref().and_then(|r| r.refused.clone()),
            }),
        )?;
        index.write_all(b"\n")?;
    }
    index.flush()?;
    let manifest = json!({
        "encoder_version": VERSION,
        "walk_version": baylee_train::cardwalk::WALK_VERSION,
        "created_by": baylee_build::short(),
        "keep": {"forced": keep.forced, "every": keep.every},
        "verification": {
            "pool": "the runs' working-card rule (sources[].working)",
            "leave_checked": false,
        },
        "ids": {
            "padding": 0,
            "card": "CardIndex + 1",
            "ledger_rows": LEDGER_ROWS,
            "unknown": UNKNOWN_ID,
            "token_base": TOKEN_BASE,
            "id_space": ID_SPACE,
        },
        "max_entities": MAX_ENTITIES,
        "max_seats": MAX_SEATS,
        "max_deck": MAX_DECK,
        "ent_cols": &ENT_COLS[..],
        "seat_cols": &SEAT_COLS[..],
        "glob_cols": &GLOB_COLS[..],
        "deck_cols": &DECK_COLS[..],
        "omni_deck_cols": ["seat_rel", "in_list", "in_library"],
        "meta_cols": META_COLS,
        "result": {"0": "loss", "1": "draw", "2": "win"},
        "masks": "winners_rel, alive_rel, team_rel: bit r is the seat r places after the deciding seat in turn order",
        "pending_kinds": PENDING_KINDS,
        "card_table": {"ids": "card_ids.i32", "features": "card_feats.f32", "rows": table_rows, "width": baylee_train::cardwalk::WIDTH},
        "sources": sources,
        "games": games.len(),
        "samples": samples,
        "left_out": {"stopped": unfinished, "refused": refused},
        "unmatched_answers": unmatched,
        "offered_objects_dropped": offered_dropped,
        "profiles": PROFILES,
        "options": {
            "file": "opt.i16 (opts × 3: head, a, b), opt_off.i64 (samples + 1); one option per row an answer can name",
            "heads": ["fixed", "entity", "ability", "pair", "attack_player", "player", "color", "subtype", "number", "mode"],
            "fixed": ["pass", "done", "yes", "no", "keep", "mulligan"],
            "verbs": ["land", "cast", "suspend", "mana", "pick"],
            "ability_slots": baylee_train::policy::ABILITY_SLOTS,
            "max_number": baylee_train::policy::MAX_NUMBER,
        },
        "shards": shards,
        "seconds": started.elapsed().as_secs_f64(),
    });
    fs::write(
        args.out.join("dataset.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    eprintln!(
        "[convert3] done: {samples} decisions from {} games in {:.1}s → {}",
        games.len(),
        started.elapsed().as_secs_f64(),
        args.out.display()
    );
    Ok(())
}
