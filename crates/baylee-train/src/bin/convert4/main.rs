//! Replays recorded games from one or more `selfplay` runs into Playernet v4
//! trajectory shards (`Step4`) + `meta.json` + `card_dsl.bin` +
//! `card_abilities.bin`.
//!
//! ```text
//! cargo run --release -p baylee-train --bin convert4 -- \
//!     --runs ~/baylee-data/runs/v4-pilot --out ~/baylee-data/datasets/v4-pilot
//! ```

// Looks cards up, so this program holds the pool (`baylee-cards-pool`).
use baylee_cards_pool as _;

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
use baylee_train::convert4::{ProducerEncoder, RolloutStepSidecar4, Step4, TeamHands};
use baylee_train::features4::{
    CoverageMatrix4, MAX_CANDIDATES, MAX_EDGES, MAX_ENTITIES, MAX_PAIRS, MAX_SEATS,
};
use baylee_train::features7::{
    CAND_FEAT_WIDTH7, ENTITY_FEAT_WIDTH7, GLOBAL_FEAT_WIDTH7, SEAT_FEAT_WIDTH7,
};
use clap::Parser;
use flate2::Compression;
use flate2::read::GzDecoder;
use serde_json::{Value, json};

#[derive(Parser, Debug)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "Independent converter modes and observation inclusion are explicit CLI flags"
)]
#[command(about = "Converts selfplay runs into Playernet v4 trajectory shards")]
struct Args {
    /// Explicit observation encoder. Archived conversion remains Encoder 7.
    #[arg(long, default_value_t = 7, value_parser = clap::value_parser!(u32).range(7..=10))]
    encoder_version: u32,
    /// Input selfplay run directories, comma-separated.
    #[arg(long, value_delimiter = ',')]
    runs: Vec<PathBuf>,
    /// Separate validation runs (for example games from held-out deck families).
    /// When supplied, every --runs game is training and every --val-runs game validation.
    #[arg(long, value_delimiter = ',')]
    val_runs: Vec<PathBuf>,
    /// The deck manifest whose train split a dataset may be played with
    /// (`manifest.v4.holdout2.json`): a game of `--runs` or `--val-runs` that
    /// seated a deck it puts in another split (test, val, holdout2), by key,
    /// cards or near, stops the conversion by name (`train_decks.rs`).
    #[arg(long)]
    train_manifest: PathBuf,
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
    /// Build a separate BC-only dataset from verified shadow/*.jsonl advice.
    #[arg(long)]
    shadow_bc: bool,
    /// Verified executed external LLM labels only; all other policy weights zero.
    #[arg(long, conflicts_with = "shadow_bc")]
    llm_bc: bool,
    /// With `--llm-bc`: leave out the games a failed conversion of the same
    /// runs could not replay on this build (its `rejected-games.json`; every
    /// entry must be a replay-audit refusal). The dataset records them.
    #[arg(long, requires = "llm_bc")]
    llm_bc_skip_unreplayable: Option<PathBuf>,
    /// Build a BC-only dataset from exported live-server game records.
    #[arg(long, conflicts_with_all = ["shadow_bc", "llm_bc"])]
    live_bc: bool,
    /// BC-only self-imitation: keep only the net decisions of the seats a
    /// `train_ops group-weights` file lists, at their weights; games it does
    /// not list are skipped.
    #[arg(long, conflicts_with_all = ["shadow_bc", "llm_bc", "live_bc"])]
    game_weights: Option<PathBuf>,
    /// BC-only search distillation: the answers searched decisions played
    /// are the policy targets; every run must have been searched.
    #[arg(long, conflicts_with_all = ["shadow_bc", "llm_bc", "live_bc", "game_weights"])]
    search_targets: bool,
    /// With `--search-targets`: the policy weight of the net's own
    /// unsearched decisions (0 leaves them out).
    #[arg(long, requires = "search_targets", default_value_t = 0.0)]
    search_unsearched_weight: f32,
    /// With `--search-targets`: the policy weight of searched decisions whose
    /// chosen action kept the policy's own answer (default 1.0 keeps all
    /// searched decisions at full weight).
    #[arg(long, requires = "search_targets", default_value_t = 1.0)]
    search_unchanged_weight: f32,
    /// With `--search-targets`: replace each step's terminal game outcome
    /// with the dense next-turn value target (or next-turn value advantage
    /// mapped to `[0, 1]`) from the game's calibrated search events.
    #[arg(long, requires = "search_targets")]
    search_turn_value_target: bool,
}

#[derive(Clone, Debug)]
struct GameRef {
    game_id: u32,
    /// Self-imitation weight per seat (`--game-weights`).
    seat_weights: Option<[f32; MAX_SEATS]>,
    producer_encoder: Option<ProducerEncoder>,
    /// What the run's producer says about teammates' hands.
    team_hands: TeamHands,
    on_policy: Option<on_policy::GameBinding>,
    shard_path: PathBuf,
    offset: u64,
    len: u64,
    sidecar: Option<Vec<RolloutStepSidecar4>>,
    search_events: Option<search_distill::GameSearchEvents>,
    validation: bool,
    shadow: Option<shadow::Source>,
    llm: Option<llm::Source>,
    /// The deck keys the game's row gives its seats, if any.
    deck_keys: Vec<Option<String>>,
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

mod boundary;
mod llm;
mod on_policy;
mod output;
mod policy_provenance;
mod research;
mod search_distill;
mod self_imitation;
mod shadow;
mod train_decks;
mod unreplayable;
use output::{GameWriter, publish_ready};

#[allow(
    clippy::too_many_lines,
    reason = "Conversion setup, workers and immutable output manifest"
)]
fn main() -> anyhow::Result<()> {
    if std::env::args_os()
        .nth(1)
        .is_some_and(|arg| arg == "research")
    {
        return research::run();
    }
    let args = Args::parse();
    if args.encoder_version == 9 && (args.shadow_bc || args.llm_bc) {
        bail!("Encoder9 shadow and LLM labels are not supported");
    }
    if args.llm_bc && !args.include_passive_obs {
        bail!("--llm-bc requires --include-passive-obs");
    }
    if args.shadow_bc && !args.include_passive_obs {
        bail!("--shadow-bc requires --include-passive-obs for complete recurrent history");
    }
    if args.game_weights.is_some() && (!args.include_passive_obs || !args.val_runs.is_empty()) {
        bail!("--game-weights requires --include-passive-obs and weighs --runs only");
    }
    if args.search_targets && (!args.include_passive_obs || !args.val_runs.is_empty()) {
        bail!("--search-targets requires --include-passive-obs and distills --runs only");
    }
    if args.out.exists() {
        bail!("{} already exists", args.out.display());
    }
    let game_weights = args
        .game_weights
        .as_deref()
        .map(|path| self_imitation::Weights::load(path, &args.runs))
        .transpose()?;
    let mut unreplayable = args
        .llm_bc_skip_unreplayable
        .as_deref()
        .map(unreplayable::Skip::load)
        .transpose()?;
    let mut search_targets = args
        .search_targets
        .then(|| {
            search_distill::Targets::with_options(
                args.search_unsearched_weight,
                args.search_unchanged_weight,
                args.search_turn_value_target,
            )
        })
        .transpose()?;
    fs::create_dir_all(&args.out)?;

    let mut games: Vec<GameRef> = Vec::new();
    let mut policy_sources = Vec::new();
    let mut shadow_seen = BTreeSet::new();
    let mut llm_seen = BTreeSet::new();
    let mut house_refusals = llm::HouseRefusals::default();
    let mut vocab_set: BTreeSet<u32> = baylee_cards::all().map(|card| card.index.get()).collect();
    vocab_set.insert(UNKNOWN_CARD);

    for run_dir in args.runs.iter().chain(&args.val_runs) {
        let run_json: Value = serde_json::from_slice(
            &fs::read(run_dir.join("run.json"))
                .with_context(|| format!("reading {}/run.json", run_dir.display()))?,
        )?;
        let on_policy = on_policy::RunBinding::load(run_dir, &run_json)?;
        if on_policy.is_some() {
            anyhow::ensure!(
                args.include_passive_obs
                    && args.val_every == 0
                    && args.val_runs.is_empty()
                    && !args.shadow_bc
                    && !args.llm_bc
                    && !args.live_bc,
                "on-policy collection requires complete unsplit passive histories and no BC relabelling"
            );
        }
        if !args.llm_bc && !args.live_bc && run_json.get("model_identity").is_some() {
            anyhow::ensure!(
                run_json["model_identity"]["encoder_version"].as_u64()
                    == Some(u64::from(args.encoder_version)),
                "Neural producer/converter encoder mismatch; probabilities cannot migrate across encoders"
            );
        }
        if let Some(targets) = &mut search_targets {
            targets
                .admit(&run_json)
                .with_context(|| format!("{}", run_dir.display()))?;
        }
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
            let policy_game = on_policy.as_ref().map(|run| run.game(&v)).transpose()?;
            let kind = v["outcome"]["kind"].as_str().unwrap_or("");
            if !matches!(kind, "won" | "team_won" | "draw") {
                continue;
            }
            let seat_weights = match &game_weights {
                Some(weights) => {
                    let record = v["record_sha256"]
                        .as_str()
                        .context("self-imitation needs record_sha256")?;
                    let Some(seats) = weights.seats(record) else {
                        continue;
                    };
                    anyhow::ensure!(
                        v.get("rollout_sidecar").is_some(),
                        "self-imitation needs the net's rollout sidecar"
                    );
                    Some(seats)
                }
                None => None,
            };
            anyhow::ensure!(
                search_targets.is_none() || v.get("rollout_sidecar").is_some(),
                "search distillation needs the rollout sidecar naming searched answers"
            );
            let shard_name = v["shard"]
                .as_str()
                .context("missing shard name in games.jsonl")?;
            let offset = v["offset"].as_u64().context("missing offset")?;
            let len = v["len"].as_u64().context("missing len")?;
            if let Some(skip) = &mut unreplayable {
                let shard = run_dir.join("records").join(shard_name).canonicalize()?;
                if skip.takes(&shard, offset, len) {
                    continue;
                }
            }
            let seed = v["seed"].as_u64().unwrap_or(0);
            let sidecar: Option<Vec<RolloutStepSidecar4>> = v
                .get("rollout_sidecar")
                .and_then(|sc| serde_json::from_value(sc.clone()).ok());
            let llm = if args.llm_bc {
                anyhow::ensure!(
                    sidecar.is_none(),
                    "LLM BC must not fabricate neural rollout provenance"
                );
                let game = v["game"].as_str().context("LLM game key")?;
                let source = llm::Source::load(
                    run_dir,
                    game,
                    &run_dir.join("records").join(shard_name),
                    offset,
                    len,
                )?;
                anyhow::ensure!(
                    llm_seen.insert(source.record_sha().to_string()),
                    "Duplicate LLM record across source namespaces"
                );
                // The corpus rule: a game the house answered more than 5 % of
                // a seat's questions of is no teacher data.
                if !house_refusals.admits(run_dir, game, &source)? {
                    continue;
                }
                Some(source)
            } else {
                None
            };
            let game_id = (games.len() + 1) as u32;
            games.push(GameRef {
                seat_weights,
                on_policy: policy_game,
                producer_encoder: if matches!(args.encoder_version, 8 | 9) && sidecar.is_some() {
                    Some(ProducerEncoder::from_run(&run_json).map_err(anyhow::Error::msg)?)
                } else {
                    None
                },
                team_hands: TeamHands::from_run(&run_json),
                deck_keys: train_decks::row_keys(&v),
                game_id,
                shard_path: if args.shadow_bc || args.llm_bc {
                    run_dir.join("records").join(shard_name).canonicalize()?
                } else {
                    run_dir.join("records").join(shard_name)
                },
                offset,
                len,
                shadow: if args.shadow_bc {
                    anyhow::ensure!(
                        sidecar.is_some(),
                        "Shadow BC requires original behavior sidecar"
                    );
                    let source = shadow::Source::load(
                        run_dir,
                        v["game"].as_str().context("Shadow game key")?,
                    )?;
                    anyhow::ensure!(
                        shadow_seen.insert(source.namespace_key()),
                        "Duplicate shadow source game"
                    );
                    Some(source)
                } else {
                    None
                },
                llm,
                sidecar,
                search_events: search_targets
                    .is_some()
                    .then(|| search_distill::GameSearchEvents::from_game(&v)),
                validation: if args.val_runs.is_empty() {
                    args.val_every > 0 && seed.is_multiple_of(args.val_every)
                } else {
                    args.val_runs.contains(run_dir)
                },
            });
        }
    }

    if args.llm_bc {
        // Written before any game converts, so a conversion that fails later
        // still says which games the corpus rule left out.
        fs::write(
            args.out.join("llm-refused-house-share.json"),
            serde_json::to_vec_pretty(&house_refusals.metadata())?,
        )?;
        eprintln!(
            "[convert4] the corpus rule refused {} LLM games (house over {} % of a seat)",
            house_refusals.count(),
            baylee_train::convert4::llm_bc::house_share::MAX_HOUSE_SHARE_PERCENT
        );
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
    // No game of a training dataset seated a deck outside the train split.
    let train_guard = baylee_train::decksplit::TrainOnly::load(
        &args.train_manifest,
        &baylee_train::working::repo_root(),
    )?;
    train_decks::check(&train_guard, &games)?;
    let is_on_policy = games.iter().any(|game| game.on_policy.is_some());
    anyhow::ensure!(
        !is_on_policy || games.iter().all(|game| game.on_policy.is_some()),
        "cannot mix certified on-policy and roleless sources"
    );
    let games_arc = Arc::new(games);
    let next_idx = Arc::new(AtomicUsize::new(0));
    let train_rows = Arc::new(Mutex::new(Vec::<(u32, String, u64)>::new()));
    let val_rows = Arc::new(Mutex::new(Vec::<(u32, String, u64)>::new()));
    let cov_total = Arc::new(Mutex::new(CoverageMatrix4::default()));
    let unencoded_total = Arc::new(AtomicUsize::new(0));
    let rejected = Arc::new(Mutex::new(Vec::<Value>::new()));
    let converted = Arc::new(AtomicUsize::new(0));
    let replay_checks = Arc::new(Mutex::new(std::collections::BTreeMap::<&str, u64>::new()));
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
        "idle_pass_weighting_revision": baylee_train::convert4::IDLE_PASS_WEIGHTING_REVISION,
        "encoder_version": args.encoder_version,
        "ppo_eligible": policy_sources.iter().all(|r| r["collection_contract"]["purpose"] == "on_policy_training"),
        "policy_provenance_version": if policy_provenance::certified_dataset(&policy_sources) && policy_sources.iter().all(|r| r["model_identity"]["encoder_version"].as_u64() == Some(u64::from(args.encoder_version))) { json!(2) } else { Value::Null },
        "package_target_steps": args.shard_steps,
        "train_decks": train_guard.metadata(),
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
        "global_feat_width": if args.encoder_version == 9 { baylee_train::features9::GLOBAL_FEAT_WIDTH9 } else { GLOBAL_FEAT_WIDTH7 },
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
    if args.shadow_bc {
        meta["shadow_bc_version"] = json!(1);
        meta["shadow_bc_contract"] = json!("verified_view_only_teacher_v1");
        meta["shadow_bc_status"] = json!("in_progress");
        meta["ppo_eligible"] = json!(false);
        meta["policy_provenance_version"] = Value::Null;
        meta["shadow_bc_sources"] = json!(
            games_arc
                .iter()
                .filter_map(|game| game.shadow.as_ref().map(|source| source.metadata(
                    game.game_id,
                    &game.shard_path,
                    game.offset,
                    game.len
                )))
                .collect::<Vec<_>>()
        );
    }
    if args.llm_bc {
        anyhow::ensure!(!games_arc.is_empty(), "No completed LLM games selected");
        meta["llm_bc_version"] = json!(1);
        meta["llm_bc_contract"] = json!("verified_executed_llm_v1");
        meta["llm_bc_status"] = json!("in_progress");
        meta["ppo_eligible"] = json!(false);
        meta["policy_provenance_version"] = Value::Null;
        meta["llm_bc_sources"] = json!(
            games_arc
                .iter()
                .filter_map(|g| g.llm.as_ref().map(|s| s.metadata(g.game_id)))
                .collect::<Vec<_>>()
        );
        if let Some(skip) = &unreplayable {
            // Before any game converts: a stale list stops the conversion.
            meta["llm_bc_skipped_unreplayable"] = skip.metadata()?;
        }
        meta["llm_bc_refused_house_share"] = house_refusals.metadata();
    }
    if args.live_bc {
        meta["live_bc_version"] = json!(1);
        meta["ppo_eligible"] = json!(false);
        meta["policy_provenance_version"] = Value::Null;
    }
    if let Some(weights) = &game_weights {
        meta["self_imitation"] = weights.metadata(games_arc.len());
        meta["ppo_eligible"] = json!(false);
    }
    if let Some(targets) = &search_targets {
        meta["search_distillation"] = targets.metadata(games_arc.len());
        meta["ppo_eligible"] = json!(false);
    }
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
        let replay_checks = replay_checks.clone();
        let include_passive = args.include_passive_obs;
        let live_bc = args.live_bc;
        let encoder_version = args.encoder_version;
        let search_weighting = search_targets
            .as_ref()
            .map(search_distill::Targets::weighting);

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
                if let Some(binding)=&gref.on_policy {binding.verify(&bytes)?;}
                let mut writer = GameWriter::new(&root, gref.validation, gref.game_id, rule_roots.clone())?;
                let mut frame_labels = gref.shadow.as_ref()
                    .map(|source| shadow::FrameLabels::new_with_encoder(&root,gref.game_id,source,encoder_version)).transpose()?;
                let mut llm_labels=gref.llm.as_ref().map(|source|llm::FrameLabels::new_with_encoder(&root,gref.game_id,source,encoder_version)).transpose()?;
                let mut search_cursor = search_weighting
                    .map(|weighting| weighting.cursor(gref.search_events.as_ref()));
                let emit = |mut step: Step4| {
                    if let Some(seats) = &gref.seat_weights { self_imitation::apply(&mut step, seats); }
                    if let Some(cursor) = &mut search_cursor { cursor.apply(&mut step); }
                    if let Some(labels) = &mut frame_labels { labels.push(&step)?; }
                    if let Some(labels)=&mut llm_labels {labels.push(&step)?;}
                    writer.push(&step).map_err(|e|e.to_string())
                };
                // Every path shows teammates the hands the run's producer showed
                // them, and refuses a team game whose producer does not say.
                let result = if let Some(source)=&gref.llm {
                    source.verify(&bytes).and_then(|mut labels|baylee_train::convert4::stream_game4_with_encoder_and_producer(&bytes,gref.game_id,true,None,None,Some(&mut labels),false,encoder_version,None,gref.team_hands,emit))
                } else if let Some(source) = &gref.shadow {
                    source.verify(&bytes,&structure_sha256).and_then(|mut labels|
                        baylee_train::convert4::stream_game4_with_encoder_and_producer(&bytes,gref.game_id,true,
                            Some(gref.sidecar.as_deref().expect("checked behavior sidecar")),Some(&mut labels),None,false,encoder_version,None,gref.team_hands,emit))
                } else if live_bc {
                    baylee_train::convert4::stream_game4_with_encoder_and_producer(&bytes,gref.game_id,include_passive,None,None,None,true,encoder_version,None,gref.team_hands,emit)
                } else {
                    baylee_train::convert4::stream_game4_with_encoder_and_producer(&bytes,gref.game_id,include_passive,gref.sidecar.as_deref(),None,None,false,encoder_version,gref.producer_encoder.as_ref(),gref.team_hands,emit)
                };
                let extracted = match result {
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
                        for sk in 0..cov.counts[pk].len() {
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
                if let Some(labels) = frame_labels { labels.finish()?; }
                if let Some(labels)=llm_labels {labels.finish()?;}
                let name = writer.finish()?;
                publish_ready(&root, gref, &name, extracted.steps, &structure_sha256)?;
                let rows = if gref.validation { &val_rows } else { &train_rows };
                rows.lock().expect("row lock").push((gref.game_id, name, extracted.steps));
                *replay_checks.lock().expect("replay check lock").entry(extracted.replay_check.name()).or_default() += 1;
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
    boundary::refuse_records_before_it(&rejected)?;

    if is_on_policy {
        anyhow::ensure!(
            rejected.is_empty(),
            "on-policy conversion rejected games; no collection published"
        );
        meta["on_policy_collection_manifest"] = json!("collection.json");
    }
    if args.shadow_bc {
        anyhow::ensure!(
            rejected.is_empty(),
            "Shadow BC conversion rejected games; no completed meta.json published"
        );
        meta["shadow_bc_status"] = json!("complete");
        let mut hashes = serde_json::Map::new();
        for game in games_arc.iter() {
            let name = format!("shadow-bc/game-{:06}.jsonl", game.game_id);
            hashes.insert(
                name.clone(),
                json!(shadow::sha(&fs::read(args.out.join(name))?)),
            );
        }
        meta["shadow_bc_label_sha256"] = Value::Object(hashes);
    }
    meta["train_steps"] = json!(train_steps);
    meta["val_steps"] = json!(val_steps);
    meta["train_shards"] = json!(train_shards);
    meta["val_shards"] = json!(val_shards);
    meta["games_converted"] = json!(converted.load(Ordering::Relaxed));
    meta["games_rejected"] = json!(rejected.len());
    meta["replay_checks"] = json!(*replay_checks.lock().expect("replay check lock"));
    meta["unencoded_decisions"] = json!(unencoded_total.load(Ordering::Relaxed));
    meta["coverage_matrix"] = json!(cov.counts);
    if args.llm_bc {
        anyhow::ensure!(
            rejected.is_empty(),
            "LLM BC rejected games; no completed meta.json published"
        );
        llm::finish_metadata(&args.out, &mut meta)?;
    }
    meta["elapsed_secs"] = json!(started.elapsed().as_secs_f64());
    fs::write(
        args.out.join("meta.json"),
        serde_json::to_vec_pretty(&meta)?,
    )?;

    if is_on_policy {
        let bindings = games_arc
            .iter()
            .map(|game| {
                (
                    game.game_id,
                    game.on_policy.as_ref().expect("checked role binding"),
                )
            })
            .collect::<Vec<_>>();
        on_policy::publish(&args.out, &meta, &bindings)?;
    }

    eprintln!(
        "[convert4] converted {} games -> {} train steps + {} val steps ({} vocab cards, {} unencoded) in {:.1}s",
        converted.load(Ordering::Relaxed),
        train_steps,
        val_steps,
        cards.len(),
        unencoded_total.load(Ordering::Relaxed),
        started.elapsed().as_secs_f64()
    );
    if args.llm_bc {
        eprintln!(
            "[convert4] LLM games refused by the corpus rule (house over {} % of a seat): {}",
            baylee_train::convert4::llm_bc::house_share::MAX_HOUSE_SHARE_PERCENT,
            house_refusals.count()
        );
    }
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

#[cfg(test)]
mod encoder_cli_tests {
    use super::*;

    #[test]
    fn encoder_selection_is_explicit_and_unsupported_versions_fail() {
        let base = ["convert4", "--out", "dataset", "--train-manifest", "m.json"];
        let with = |extra: &[&'static str]| Args::try_parse_from(base.iter().chain(extra));
        let default = with(&[]).unwrap();
        assert_eq!(default.encoder_version, 7);
        let payment = with(&["--encoder-version", "8"]).unwrap();
        assert_eq!(payment.encoder_version, 8);
        let announcement = with(&["--encoder-version", "9"]).unwrap();
        assert_eq!(announcement.encoder_version, 9);
        assert!(with(&["--encoder-version", "11"]).is_err());
    }

    /// No training conversion runs without the manifest that guards its decks.
    #[test]
    fn a_conversion_names_its_train_manifest() {
        assert!(Args::try_parse_from(["convert4", "--out", "dataset"]).is_err());
    }
}
