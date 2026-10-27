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
use baylee_train::convert4::{RolloutStepSidecar4, Step4, stream_game4};
use baylee_train::features4::{
    CoverageMatrix4, MAX_CANDIDATES, MAX_EDGES, MAX_ENTITIES, MAX_PAIRS, MAX_SEATS,
};
use baylee_train::features7::{
    CAND_FEAT_WIDTH7, ENCODER_VERSION7, ENTITY_FEAT_WIDTH7, GLOBAL_FEAT_WIDTH7, SEAT_FEAT_WIDTH7,
};
use clap::Parser;
use flate2::Compression;
use flate2::read::GzDecoder;
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
    /// Reproducibly mix game scheduling across source runs (0 keeps input order).
    #[arg(long, default_value_t = 0)]
    schedule_seed: u64,
    /// Suggested frames per consumer package; games are never split.
    #[arg(
        long = "package-steps",
        alias = "shard-steps",
        default_value_t = 250_000
    )]
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

fn journal_worker(path: &Path, game: &GameRef, status: &str) -> anyhow::Result<()> {
    let temporary = path.with_extension("partial");
    fs::write(
        &temporary,
        serde_json::to_vec(&json!({
            "status": status, "game_id": game.game_id, "shard": game.shard_path,
            "offset": game.offset, "len": game.len, "validation": game.validation,
        }))?,
    )?;
    fs::rename(temporary, path)?;
    Ok(())
}

mod output;
mod policy_provenance;
use output::{GameWriter, publish_ready};

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
    let mut policy_sources = Vec::new();
    let mut vocab_set: BTreeSet<u32> = baylee_cards::all().map(|card| card.index.get()).collect();
    vocab_set.insert(UNKNOWN_CARD);

    for run_dir in args.runs.iter().chain(&args.val_runs) {
        let run_json: Value = serde_json::from_slice(
            &fs::read(run_dir.join("run.json"))
                .with_context(|| format!("reading {}/run.json", run_dir.display()))?,
        )?;
        policy_sources.push(run_json.clone());
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
    if args.schedule_seed != 0 {
        use sha2::{Digest as _, Sha256};
        games.sort_by_cached_key(|game| {
            let mut hash = Sha256::new();
            hash.update(args.schedule_seed.to_le_bytes());
            hash.update(game.game_id.to_le_bytes());
            <[u8; 32]>::from(hash.finalize())
        });
    }

    let threads = match args.threads {
        0 => std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get),
        n => n,
    };

    let structure = baylee_train::cardstructure::catalog(
        &baylee_cards::all()
            .map(|c| c.index.get())
            .collect::<Vec<_>>(),
    )?;
    let rule_roots = Arc::new(structure.roots.clone());
    let started = Instant::now();
    let games_arc = Arc::new(games);
    let next_idx = Arc::new(AtomicUsize::new(0));
    let train_rows = Arc::new(Mutex::new(Vec::<(u32, String, u64)>::new()));
    let val_rows = Arc::new(Mutex::new(Vec::<(u32, String, u64)>::new()));
    let cov_total = Arc::new(Mutex::new(CoverageMatrix4::default()));
    let unencoded_total = Arc::new(AtomicUsize::new(0));
    let rejected = Arc::new(Mutex::new(Vec::<Value>::new()));
    let converted = Arc::new(AtomicUsize::new(0));
    let worker_dir = args.out.join("workers");
    fs::create_dir_all(&worker_dir)?;

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

    let structure_sha256 = baylee_train::cardstructure::digest(&structure)?;
    fs::write(
        args.out.join("card_structure.json"),
        serde_json::to_vec(&structure)?,
    )?;

    let mut meta = json!({
        "encoder_version": ENCODER_VERSION7,
        "policy_provenance_version": if policy_provenance::certified_dataset(&policy_sources) { json!(1) } else { Value::Null },
        "package_target_steps": args.shard_steps,
        "streaming_frames_per_worker": MAX_SEATS,
        "trajectory_version": if args.include_passive_obs { 2 } else { 1 },
        "sampling_version": 2,
        "dsl_version": DSL_VERSION,
        "structure_version": 1,
        "structure_sha256": structure_sha256,
        "card_dsl_width": CARD_DSL_WIDTH,
        "ability_dsl_width": ABILITY_DSL_WIDTH,
        "max_abilities": MAX_ABILITIES,
        "entity_feat_width": ENTITY_FEAT_WIDTH7,
        "seat_feat_width": SEAT_FEAT_WIDTH7,
        "global_feat_width": GLOBAL_FEAT_WIDTH7,
        "cand_feat_width": CAND_FEAT_WIDTH7,
        "max_seats": MAX_SEATS,
        "max_entities": MAX_ENTITIES,
        "max_edges": MAX_EDGES,
        "max_candidates": MAX_CANDIDATES,
        "max_pairs": MAX_PAIRS,
        "cards": cards,
        "token_count": baylee_cards::tokens::ALL.len(),
        "games_requested": games_arc.len(),
        "games_converted": 0,
        "games_rejected": 0,
        "source_runs": args.runs,
        "validation_runs": args.val_runs,
        "train_steps": 0,
        "val_steps": 0,
        "train_shards": [],
        "val_shards": [],
        "unencoded_decisions": 0,
        "coverage_matrix": CoverageMatrix4::default().counts,
        "elapsed_secs": 0.0,
    });
    fs::write(
        args.out.join("dataset_header.json"),
        serde_json::to_vec_pretty(&meta)?,
    )?;
    fs::create_dir_all(args.out.join("ready"))?;

    let mut handles = Vec::new();
    for worker_id in 0..threads {
        let journal = worker_dir.join(format!("w{worker_id:02}.json"));
        let games_arc = games_arc.clone();
        let next_idx = next_idx.clone();
        let train_rows = train_rows.clone();
        let val_rows = val_rows.clone();
        let root = args.out.clone();
        let rule_roots = rule_roots.clone();
        let structure_sha256 = structure_sha256.clone();
        let cov_total = cov_total.clone();
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
                // Persist the exact record member before replay: an abort or
                // memory deadline must still identify the in-flight game.
                journal_worker(&journal, gref, "replaying")?;
                let bytes = read_member(&gref.shard_path, gref.offset, gref.len)?;
                let mut writer = GameWriter::new(&root, gref.validation, gref.game_id, rule_roots.clone())?;
                let extracted = match stream_game4(
                    &bytes,
                    gref.game_id,
                    include_passive,
                    gref.sidecar.as_deref(),
                    |step| writer.push(&step).map_err(|e| e.to_string()),
                ) {
                    Ok(ex) => ex,
                    Err(e) => {
                        eprintln!("[convert4] quarantining game {}: {e}", gref.game_id);
                        rejected.lock().expect("rejection lock").push(json!({
                            "game_id": gref.game_id, "shard": gref.shard_path,
                            "offset": gref.offset, "len": gref.len, "reason": e,
                        }));
                        journal_worker(&journal, gref, "rejected")?;
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
                if extracted.unencoded_decisions > 0 {
                    rejected.lock().expect("rejection lock").push(json!({
                        "game_id": gref.game_id, "reason": "unencoded decisions", "count": extracted.unencoded_decisions,
                    }));
                    journal_worker(&journal, gref, "rejected")?;
                    continue;
                }
                let name = writer.finish()?;
                publish_ready(&root, gref, &name, extracted.steps, &structure_sha256)?;
                let rows = if gref.validation { &val_rows } else { &train_rows };
                rows.lock().expect("row lock").push((gref.game_id, name, extracted.steps));
                journal_worker(&journal, gref, "complete")?;
                let complete = converted.fetch_add(1, Ordering::Relaxed) + 1;
                if complete.is_multiple_of(16) || complete == games_arc.len() {
                    eprintln!("[convert4] completed {complete}/{} games", games_arc.len());
                }
            }
            Ok(())
        }));
    }

    for h in handles {
        h.join().expect("worker panicked")?;
    }

    let finish_rows = |rows: &Mutex<Vec<(u32, String, u64)>>| {
        let mut rows = rows.lock().expect("rows lock");
        rows.sort_by_key(|row| row.0);
        (
            rows.iter().map(|row| row.2).sum::<u64>(),
            rows.iter().map(|row| row.1.clone()).collect::<Vec<_>>(),
        )
    };
    let (train_steps, train_shards) = finish_rows(&train_rows);
    let (val_steps, val_shards) = finish_rows(&val_rows);
    let cov = cov_total.lock().expect("cov lock").clone();
    let rejected = rejected.lock().expect("rejection lock");
    fs::write(
        args.out.join("rejected-games.json"),
        serde_json::to_vec_pretty(&*rejected)?,
    )?;

    meta["train_steps"] = json!(train_steps);
    meta["val_steps"] = json!(val_steps);
    meta["train_shards"] = json!(train_shards);
    meta["val_shards"] = json!(val_shards);
    meta["games_converted"] = json!(converted.load(Ordering::Relaxed));
    meta["games_rejected"] = json!(rejected.len());
    meta["unencoded_decisions"] = json!(unencoded_total.load(Ordering::Relaxed));
    meta["coverage_matrix"] = json!(cov.counts);
    meta["elapsed_secs"] = json!(started.elapsed().as_secs_f64());
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
