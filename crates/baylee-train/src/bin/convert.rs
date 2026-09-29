//! Turns self-play runs into a dataset the trainer reads.
//!
//! ```text
//! cargo run --profile selfplay -p baylee-train --bin convert -- \
//!     --runs ~/baylee-data/runs/r002 --out ~/baylee-data/datasets/d002
//! ```
//!
//! Only games that ended by their rules are converted; a record that no
//! longer replays to its hashes (a card changed since) is refused whole and
//! counted. Each worker writes one shard, a directory of flat little-endian
//! column files that `numpy.memmap` reads as they are:
//!
//! - `ent_card.i32`, `ent_feat.i16` (entities × `ENT_COLS`), `ent_off.i64`
//!   (samples + 1): the actor's entities, sample `k` owning rows
//!   `ent_off[k]..ent_off[k+1]`;
//! - `glob.i16` (samples × `GLOB_WIDTH`), `meta.i32` (samples × `META_COLS`);
//! - `omni_card.i32`, `omni_tag.i16`, `omni_pos.i16`, `omni_off.i64`: the
//!   hidden half, for the training-only critic and auxiliary heads.
//!
//! `dataset.json` names every column, the encoder version, and each source
//! run's build and working-card hash; `games.jsonl` maps the `game` column
//! back to its run.

use std::fs::{self, File};
use std::io::{BufRead as _, BufReader, BufWriter, Read as _, Seek as _, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use anyhow::{Context as _, bail};
use baylee_train::convert::{Converted, Keep, Refused, convert};
use baylee_train::features::{
    ENCODER_VERSION, ENT_COLS, GLOB_HEAD, GLOB_WIDTH, ID_SPACE, LEDGER_ROWS, MAX_ENTITIES,
    OMNI_LIBRARY_TOP, PENDING_KINDS, SEAT_COLS, SEATS, TOKEN_BASE, UNKNOWN_ID,
};
use clap::Parser;
use flate2::read::GzDecoder;
use serde_json::{Value, json};

/// Columns of `meta.i32`.
const META_COLS: [&str; 9] = [
    "game",
    "seat",
    "n",
    "turn",
    "result",
    "final_turn",
    "pending_kind",
    "n_options",
    "entities_dropped",
];

#[derive(Parser, Debug)]
#[command(about = "Turns self-play runs into a dataset the trainer reads")]
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
}

/// One worker's column files.
struct Shard {
    dir: PathBuf,
    ent_card: BufWriter<File>,
    ent_feat: BufWriter<File>,
    ent_off: BufWriter<File>,
    glob: BufWriter<File>,
    meta: BufWriter<File>,
    omni_card: BufWriter<File>,
    omni_tag: BufWriter<File>,
    omni_pos: BufWriter<File>,
    omni_off: BufWriter<File>,
    samples: u64,
    entities: u64,
    omni: u64,
}

fn put<W: Write>(w: &mut W, bytes: &[u8]) -> std::io::Result<()> {
    w.write_all(bytes)
}

impl Shard {
    fn create(dir: PathBuf) -> anyhow::Result<Self> {
        fs::create_dir_all(&dir)?;
        let open = |name: &str| -> anyhow::Result<BufWriter<File>> {
            Ok(BufWriter::with_capacity(
                1 << 20,
                File::create(dir.join(name))?,
            ))
        };
        let mut shard = Self {
            ent_card: open("ent_card.i32")?,
            ent_feat: open("ent_feat.i16")?,
            ent_off: open("ent_off.i64")?,
            glob: open("glob.i16")?,
            meta: open("meta.i32")?,
            omni_card: open("omni_card.i32")?,
            omni_tag: open("omni_tag.i16")?,
            omni_pos: open("omni_pos.i16")?,
            omni_off: open("omni_off.i64")?,
            dir,
            samples: 0,
            entities: 0,
            omni: 0,
        };
        put(&mut shard.ent_off, &0_i64.to_le_bytes())?;
        put(&mut shard.omni_off, &0_i64.to_le_bytes())?;
        Ok(shard)
    }

    fn write(&mut self, game: u32, converted: &Converted) -> anyhow::Result<()> {
        for s in &converted.samples {
            for card in &s.actor.cards {
                put(&mut self.ent_card, &card.to_le_bytes())?;
            }
            for row in &s.actor.rows {
                for v in row {
                    put(&mut self.ent_feat, &v.to_le_bytes())?;
                }
            }
            self.entities += s.actor.cards.len() as u64;
            put(&mut self.ent_off, &(self.entities as i64).to_le_bytes())?;
            for v in &s.actor.globals {
                put(&mut self.glob, &v.to_le_bytes())?;
            }
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
                i64::from(s.actor.kind),
                i64::from(s.actor.options),
                s.actor.dropped as i64,
            ];
            for v in meta {
                let v = i32::try_from(v).unwrap_or(i32::MAX);
                put(&mut self.meta, &v.to_le_bytes())?;
            }
            for ((card, tag), pos) in s.omni.cards.iter().zip(&s.omni.tags).zip(&s.omni.positions) {
                put(&mut self.omni_card, &card.to_le_bytes())?;
                put(&mut self.omni_tag, &tag.to_le_bytes())?;
                put(&mut self.omni_pos, &pos.to_le_bytes())?;
            }
            self.omni += s.omni.cards.len() as u64;
            put(&mut self.omni_off, &(self.omni as i64).to_le_bytes())?;
            self.samples += 1;
        }
        Ok(())
    }

    fn finish(mut self) -> anyhow::Result<Value> {
        for w in [
            &mut self.ent_card,
            &mut self.ent_feat,
            &mut self.ent_off,
            &mut self.glob,
            &mut self.meta,
            &mut self.omni_card,
            &mut self.omni_tag,
            &mut self.omni_pos,
            &mut self.omni_off,
        ] {
            w.flush()?;
        }
        Ok(json!({
            "dir": self.dir.file_name().and_then(|n| n.to_str()),
            "samples": self.samples,
            "entities": self.entities,
            "omni": self.omni,
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
            if kind == "won" || kind == "draw" {
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
        }));
    }
    let threads = match args.threads {
        0 => std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get),
        n => n,
    };
    fs::create_dir_all(&args.out)?;
    eprintln!(
        "[convert] {} finished games from {} run(s) on {threads} threads ({unfinished} stopped games left out)",
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
                let report = match convert(&record, keep) {
                    Ok(converted) => {
                        shard.write(game.id, &converted)?;
                        Report {
                            id: game.id,
                            samples: converted.samples.len(),
                            refused: None,
                        }
                    }
                    Err(why) => Report {
                        id: game.id,
                        samples: 0,
                        refused: Some(match why {
                            Refused::Diverged(_) => "diverged".to_owned(),
                            other => format!("{other:?}"),
                        }),
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
    loop {
        match rx.recv_timeout(Duration::from_secs(1)) {
            Ok(report) => {
                done += 1;
                samples += report.samples;
                if let Some(why) = &report.refused {
                    *refused.entry(why.clone()).or_default() += 1;
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
                "[convert] {done}/{} games · {samples} decisions · {:.0} games/s · refused {refused:?}",
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
                "outcome": line["outcome"],
                "samples": report.as_ref().map_or(0, |r| r.samples),
                "refused": report.as_ref().and_then(|r| r.refused.clone()),
            }),
        )?;
        index.write_all(b"\n")?;
    }
    index.flush()?;
    let manifest = json!({
        "encoder_version": ENCODER_VERSION,
        "created_by": baylee_build::short(),
        "keep": {"forced": keep.forced, "every": keep.every},
        "ids": {
            "padding": 0,
            "card": "CardIndex + 1",
            "ledger_rows": LEDGER_ROWS,
            "unknown": UNKNOWN_ID,
            "token_base": TOKEN_BASE,
            "id_space": ID_SPACE,
        },
        "max_entities": MAX_ENTITIES,
        "ent_cols": &ENT_COLS[..],
        "glob_cols": GLOB_HEAD
            .iter()
            .map(|c| (*c).to_owned())
            .chain((0..SEATS).flat_map(|s| SEAT_COLS.iter().map(move |c| format!("seat{s}_{c}"))))
            .collect::<Vec<_>>(),
        "glob_width": GLOB_WIDTH,
        "meta_cols": META_COLS,
        "result": {"0": "loss", "1": "draw", "2": "win"},
        "pending_kinds": PENDING_KINDS,
        "omni": {"tag": "seat_rel * 16 + what (1 hand, 2 library top)", "library_top": OMNI_LIBRARY_TOP},
        "sources": sources,
        "games": games.len(),
        "samples": samples,
        "left_out": {"stopped": unfinished, "refused": refused},
        "shards": shards,
        "seconds": started.elapsed().as_secs_f64(),
    });
    fs::write(
        args.out.join("dataset.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    eprintln!(
        "[convert] done: {samples} decisions from {} games in {:.1}s → {}",
        games.len(),
        started.elapsed().as_secs_f64(),
        args.out.display()
    );
    Ok(())
}
