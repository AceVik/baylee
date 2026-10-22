//! Replays recorded games from one or more `selfplay` runs into Playernet v4
//! trajectory shards (`Step4`) + `meta.json` + `card_dsl.bin` +
//! `card_abilities.bin`.
//!
//! ```text
//! cargo run --release -p baylee-train --bin convert4 -- \
//!     --runs ~/baylee-data/runs/v4-pilot --out ~/baylee-data/datasets/v4-pilot
//! ```

use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use anyhow::{Context as _, bail};
use baylee_train::carddsl::{
    ABILITY_DSL_WIDTH, CARD_DSL_WIDTH, DSL_VERSION, MAX_ABILITIES, UNKNOWN_CARD,
    card_ability_table, card_back_summary_table, card_summary_table, token_summary_table,
};
use baylee_train::convert4::{RolloutStepSidecar4, extract_game4_with_sidecar};
use baylee_train::features4::{
    CAND_FEAT_WIDTH, CoverageMatrix4, ENCODER_VERSION, ENTITY_FEAT_WIDTH, GLOBAL_FEAT_WIDTH,
    MAX_CANDIDATES, MAX_EDGES, MAX_ENTITIES, MAX_PAIRS, MAX_SEATS, SEAT_FEAT_WIDTH,
};
use clap::Parser;
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use serde_json::{Value, json};

#[derive(Parser, Debug)]
#[command(about = "Converts selfplay runs into Playernet v4 trajectory shards")]
struct Args {
    /// Input selfplay run directories, comma-separated.
    #[arg(long, value_delimiter = ',')]
    runs: Vec<PathBuf>,
    /// Separate validation runs (for example games from held-out deck families).
    /// When supplied, every --runs game is training and every --val-runs game validation.
    #[arg(long, value_delimiter = ',')]
    val_runs: Vec<PathBuf>,
    /// Output dataset directory; must not exist yet.
    #[arg(long)]
    out: PathBuf,
    /// Worker threads (0 = available cores).
    #[arg(long, default_value_t = 0)]
    threads: usize,
    /// Steps per output `.jsonl.gz` shard.
    #[arg(long, default_value_t = 25_000)]
    shard_steps: usize,
    /// Validation fraction (`1 / val_every` games go to `val/`).
    #[arg(long, default_value_t = 10)]
    val_every: u64,
    /// Include passive observation frames when another seat acts.
    #[arg(long)]
    include_passive_obs: bool,
}

#[derive(Clone, Debug)]
struct GameRef {
    game_id: u32,
    shard_path: PathBuf,
    offset: u64,
    len: u64,
    sidecar: Option<Vec<RolloutStepSidecar4>>,
    validation: bool,
}

fn read_member(path: &Path, offset: u64, len: u64) -> anyhow::Result<Vec<u8>> {
    let mut file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    file.seek(SeekFrom::Start(offset))?;
    let slice = file.take(len);
    let mut gz = GzDecoder::new(slice);
    let mut out = Vec::new();
    gz.read_to_end(&mut out)?;
    Ok(out)
}

struct ShardWriter {
    dir: PathBuf,
    prefix: &'static str,
    shard_steps: usize,
    shard_idx: usize,
    cur_steps: usize,
    enc: Option<GzEncoder<BufWriter<File>>>,
    shards_written: Vec<String>,
    total_steps: u64,
}

impl ShardWriter {
    fn new(dir: PathBuf, prefix: &'static str, shard_steps: usize) -> anyhow::Result<Self> {
        fs::create_dir_all(&dir)?;
        Ok(Self {
            dir,
            prefix,
            shard_steps,
            shard_idx: 0,
            cur_steps: 0,
            enc: None,
            shards_written: Vec::new(),
            total_steps: 0,
        })
    }

    fn write_raw_batch(&mut self, raw_jsonl: &[u8], step_count: usize) -> anyhow::Result<()> {
        if step_count == 0 {
            return Ok(());
        }
        if self.enc.is_none() || self.cur_steps >= self.shard_steps {
            if let Some(old) = self.enc.take() {
                old.finish()?.flush()?;
            }
            let name = format!("{}-{:04}.jsonl.gz", self.prefix, self.shard_idx);
            self.shard_idx += 1;
            self.cur_steps = 0;
            let file = File::create(self.dir.join(&name))?;
            self.enc = Some(GzEncoder::new(BufWriter::new(file), Compression::fast()));
            self.shards_written.push(name);
        }
        let enc = self.enc.as_mut().expect("opened above");
        enc.write_all(raw_jsonl)?;
        self.cur_steps += step_count;
        self.total_steps += step_count as u64;
        Ok(())
    }

    fn finish(mut self) -> anyhow::Result<(u64, Vec<String>)> {
        if let Some(old) = self.enc.take() {
            old.finish()?.flush()?;
        }
        Ok((self.total_steps, self.shards_written))
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "Conversion setup, workers and immutable output manifest"
)]
fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    if args.out.exists() {
        bail!("{} already exists", args.out.display());
    }
    fs::create_dir_all(&args.out)?;

    let mut games: Vec<GameRef> = Vec::new();
    let mut vocab_set: BTreeSet<u32> = baylee_cards::all().map(|card| card.index.get()).collect();
    vocab_set.insert(UNKNOWN_CARD);

    for run_dir in args.runs.iter().chain(&args.val_runs) {
        let run_json: Value = serde_json::from_slice(
            &fs::read(run_dir.join("run.json"))
                .with_context(|| format!("reading {}/run.json", run_dir.display()))?,
        )?;
        if let Some(decks) = run_json["decks"].as_array() {
            for d in decks {
                if let Some(cards) = d["cards"].as_array() {
                    for c in cards {
                        if let Some(cid) = c.as_u64() {
                            vocab_set.insert(cid as u32);
                        }
                    }
                }
            }
        }
        let games_file = File::open(run_dir.join("games.jsonl"))?;
        for line in BufReader::new(games_file).lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let v: Value = serde_json::from_str(&line)?;
            let kind = v["outcome"]["kind"].as_str().unwrap_or("");
            if !matches!(kind, "won" | "team_won" | "draw") {
                continue;
            }
            let shard_name = v["shard"]
                .as_str()
                .context("missing shard name in games.jsonl")?;
            let offset = v["offset"].as_u64().context("missing offset")?;
            let len = v["len"].as_u64().context("missing len")?;
            let seed = v["seed"].as_u64().unwrap_or(0);
            let sidecar: Option<Vec<RolloutStepSidecar4>> = v
                .get("rollout_sidecar")
                .and_then(|sc| serde_json::from_value(sc.clone()).ok());
            let game_id = (games.len() + 1) as u32;
            games.push(GameRef {
                game_id,
                shard_path: run_dir.join("records").join(shard_name),
                offset,
                len,
                sidecar,
                validation: if args.val_runs.is_empty() {
                    args.val_every > 0 && seed.is_multiple_of(args.val_every)
                } else {
                    args.val_runs.contains(run_dir)
                },
            });
        }
    }

    if games.is_empty() {
        bail!("no finished games found across {:?}", args.runs);
    }

    let threads = match args.threads {
        0 => std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get),
        n => n,
    };

    let started = Instant::now();
    let games_arc = Arc::new(games);
    let next_idx = Arc::new(AtomicUsize::new(0));
    let train_writer = Arc::new(Mutex::new(ShardWriter::new(
        args.out.join("train"),
        "steps",
        args.shard_steps,
    )?));
    let val_writer = Arc::new(Mutex::new(ShardWriter::new(
        args.out.join("val"),
        "steps",
        args.shard_steps,
    )?));
    let cov_total = Arc::new(Mutex::new(CoverageMatrix4::default()));
    let vocab_extra = Arc::new(Mutex::new(BTreeSet::<u32>::new()));
    let unencoded_total = Arc::new(AtomicUsize::new(0));
    let rejected = Arc::new(Mutex::new(Vec::<Value>::new()));
    let converted = Arc::new(AtomicUsize::new(0));

    let mut handles = Vec::new();
    for _ in 0..threads {
        let games_arc = games_arc.clone();
        let next_idx = next_idx.clone();
        let train_writer = train_writer.clone();
        let val_writer = val_writer.clone();
        let cov_total = cov_total.clone();
        let vocab_extra = vocab_extra.clone();
        let unencoded_total = unencoded_total.clone();
        let rejected = rejected.clone();
        let converted = converted.clone();
        let include_passive = args.include_passive_obs;

        handles.push(std::thread::spawn(move || -> anyhow::Result<()> {
            loop {
                let idx = next_idx.fetch_add(1, Ordering::Relaxed);
                let Some(gref) = games_arc.get(idx) else {
                    break;
                };
                let bytes = read_member(&gref.shard_path, gref.offset, gref.len)?;
                let extracted = match extract_game4_with_sidecar(
                    &bytes,
                    gref.game_id,
                    include_passive,
                    gref.sidecar.as_deref(),
                ) {
                    Ok(ex) => ex,
                    Err(e) => {
                        eprintln!("[convert4] quarantining game {}: {e}", gref.game_id);
                        rejected.lock().expect("rejection lock").push(json!({
                            "game_id": gref.game_id, "shard": gref.shard_path,
                            "offset": gref.offset, "len": gref.len, "reason": e,
                        }));
                        continue;
                    }
                };
                unencoded_total
                    .fetch_add(extracted.unencoded_decisions as usize, Ordering::Relaxed);
                {
                    let mut cov = cov_total.lock().expect("cov lock");
                    for pk in 0..extracted.coverage.counts.len() {
                        for sk in 0..4 {
                            cov.counts[pk][sk] += extracted.coverage.counts[pk][sk];
                        }
                    }
                }
                let mut local_cards = BTreeSet::new();
                for st in &extracted.steps {
                    for &cid in &st.state.entity_card_ids {
                        if cid != UNKNOWN_CARD {
                            local_cards.insert(cid);
                        }
                    }
                    for &cid in &st.cand_card_id {
                        if cid != UNKNOWN_CARD {
                            local_cards.insert(cid);
                        }
                    }
                }
                if !local_cards.is_empty() {
                    vocab_extra.lock().expect("vocab lock").extend(local_cards);
                }

                let mut raw_buf = Vec::with_capacity(extracted.steps.len() * 2048);
                for st in &extracted.steps {
                    serde_json::to_writer(&mut raw_buf, st)?;
                    raw_buf.push(b'\n');
                }
                let is_val = gref.validation;
                let writer = if is_val { &val_writer } else { &train_writer };
                let mut w = writer.lock().expect("writer lock");
                w.write_raw_batch(&raw_buf, extracted.steps.len())?;
                converted.fetch_add(1, Ordering::Relaxed);
            }
            Ok(())
        }));
    }

    for h in handles {
        h.join().expect("worker panicked")?;
    }

    vocab_set.extend(vocab_extra.lock().expect("vocab lock").iter().copied());
    let cards: Vec<u32> = vocab_set.into_iter().collect();

    // Write card_dsl.bin (front), card_dsl_back.bin (back), token_dsl.bin (tokens), and card_abilities.bin
    let summary_table = card_summary_table(&cards);
    let back_summary_table = card_back_summary_table(&cards);
    let tok_table = token_summary_table();
    let ability_table = card_ability_table(&cards);
    write_f32_bin(&args.out.join("card_dsl.bin"), &summary_table)?;
    write_f32_bin(&args.out.join("card_dsl_back.bin"), &back_summary_table)?;
    write_f32_bin(&args.out.join("token_dsl.bin"), &tok_table)?;
    write_f32_bin(&args.out.join("card_abilities.bin"), &ability_table)?;

    let (train_steps, train_shards) = Arc::try_unwrap(train_writer)
        .map_err(|_| anyhow::anyhow!("train_writer still shared"))?
        .into_inner()?
        .finish()?;
    let (val_steps, val_shards) = Arc::try_unwrap(val_writer)
        .map_err(|_| anyhow::anyhow!("val_writer still shared"))?
        .into_inner()?
        .finish()?;
    let cov = cov_total.lock().expect("cov lock").clone();
    let rejected = rejected.lock().expect("rejection lock");
    fs::write(
        args.out.join("rejected-games.json"),
        serde_json::to_vec_pretty(&*rejected)?,
    )?;

    let meta = json!({
        "encoder_version": ENCODER_VERSION,
        "trajectory_version": if args.include_passive_obs { 2 } else { 1 },
        "sampling_version": 2,
        "dsl_version": DSL_VERSION,
        "card_dsl_width": CARD_DSL_WIDTH,
        "ability_dsl_width": ABILITY_DSL_WIDTH,
        "max_abilities": MAX_ABILITIES,
        "entity_feat_width": ENTITY_FEAT_WIDTH,
        "seat_feat_width": SEAT_FEAT_WIDTH,
        "global_feat_width": GLOBAL_FEAT_WIDTH,
        "cand_feat_width": CAND_FEAT_WIDTH,
        "max_seats": MAX_SEATS,
        "max_entities": MAX_ENTITIES,
        "max_edges": MAX_EDGES,
        "max_candidates": MAX_CANDIDATES,
        "max_pairs": MAX_PAIRS,
        "cards": cards,
        "token_count": baylee_cards::tokens::ALL.len(),
        "games_requested": games_arc.len(),
        "games_converted": converted.load(Ordering::Relaxed),
        "games_rejected": rejected.len(),
        "source_runs": args.runs,
        "validation_runs": args.val_runs,
        "train_steps": train_steps,
        "val_steps": val_steps,
        "train_shards": train_shards,
        "val_shards": val_shards,
        "unencoded_decisions": unencoded_total.load(Ordering::Relaxed),
        "coverage_matrix": cov.counts,
        "elapsed_secs": started.elapsed().as_secs_f64(),
    });
    fs::write(
        args.out.join("meta.json"),
        serde_json::to_vec_pretty(&meta)?,
    )?;

    eprintln!(
        "[convert4] converted {} games -> {} train steps + {} val steps ({} vocab cards, {} unencoded) in {:.1}s",
        converted.load(Ordering::Relaxed),
        train_steps,
        val_steps,
        cards.len(),
        unencoded_total.load(Ordering::Relaxed),
        started.elapsed().as_secs_f64()
    );
    Ok(())
}

fn write_f32_bin(path: &Path, data: &[f32]) -> anyhow::Result<()> {
    let mut f = BufWriter::new(File::create(path)?);
    for &v in data {
        f.write_all(&v.to_le_bytes())?;
    }
    f.flush()?;
    Ok(())
}
