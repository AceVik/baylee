//! Seed- and deck-mirrored ("Duplicate Magic") in-process evaluation runner.
//!
//! Each deal pairs two profiles `(A, B)` on a fixed 60-card Constructed deck
//! matchup `(Deck 0, Deck 1)` and a fixed 64-bit RNG `seed`, then plays two
//! legs:
//! - Leg 0: `Seat 0 = A (Deck 0)` vs `Seat 1 = B (Deck 1)` with `seed`
//! - Leg 1: `Seat 0 = B (Deck 0)` vs `Seat 1 = A (Deck 1)` with `seed`
//!
//! Both legs deal byte-for-byte identical shuffled libraries and opening hands
//! via [`Session::new_recorded`], driving real [`SeatCore`] + [`Mind`] bridges
//! in-process without weakening the internet-facing gateway's server-chosen
//! shuffle seed.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result, bail, ensure};
use baylee_cards::decks::{LoadedDeck, preset_for};
use baylee_core::ids::PlayerId;
use baylee_core::preset::SeatController;
use baylee_engine::choice::PlayerAction;
use baylee_engine::win::{GameResult, Victor};
use baylee_gamehost::session::Session;
use baylee_protocol::v1::{self, Envelope};
use baylee_seat::config::{Overrides, Paths, SeatSettings, plan};
use baylee_seat::deck::list_of;
use baylee_seat::llm;
use baylee_seat::lobby::seat_name;
use baylee_seat::transcript::Transcript;
use baylee_seat::{BridgeConfig, DeckList, Mind, Request, SeatCore, Step};
use baylee_train::housedeck::HouseDeck;
use futures_util::future::join_all;
use prost::Message as _;
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use uuid::Uuid;

use crate::audit::{read_jsonl, write_atomic_json};
use crate::controller_lock;

/// Arguments for `llm_collector4 duplicate`.
#[derive(Clone, Debug)]
pub struct DuplicateConfig {
    pub root: PathBuf,
    pub profiles_config: PathBuf,
    pub profiles: Vec<String>,
    pub decks_dir: PathBuf,
    pub deals_per_pair: usize,
    pub base_seed: u64,
    pub concurrency: usize,
    pub call_timeout_seconds: u64,
    pub max_actions: u32,
}

#[derive(Clone, Debug)]
pub(crate) struct EvalDeck {
    pub(crate) key: String,
    pub(crate) path: PathBuf,
    pub(crate) sha256: String,
    pub(crate) loaded: LoadedDeck,
    pub(crate) list: DeckList,
}

#[derive(Clone, Debug)]
pub(crate) struct LegSpec {
    pub(crate) attempt_index: usize,
    pub(crate) deal_index: usize,
    pub(crate) deal_id: String,
    pub(crate) leg: u8,
    pub(crate) seed: u64,
    pub(crate) pair_profiles: [String; 2],
    pub(crate) seat_profiles: [String; 2],
    pub(crate) decks: [EvalDeck; 2],
    pub(crate) swiss_round: Option<usize>,
    pub(crate) swiss_table: Option<usize>,
}

/// Optional strict tournament and zero-fallback enforcement for in-process legs.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct TableRules {
    /// Enforce tournament Constructed rules (`starting_life = 20`, 0 free mulligans,
    /// `auto_discard_empty_optional_triggers = false`).
    pub(crate) tournament_constructed: bool,
    /// Abort immediately if any seat ever answers via `By::House` or `By::Least`.
    pub(crate) strict_zero_house: bool,
}

/// Shared execution settings for running a single mirrored leg.
#[derive(Clone, Debug)]
pub(crate) struct LegExecParams {
    pub(crate) profiles_config: PathBuf,
    pub(crate) call_timeout_seconds: u64,
    pub(crate) max_actions: u32,
    pub(crate) rules: TableRules,
}

/// Runs all mirrored duplicate legs under `cfg.root`, skipping already-admitted attempts.
pub async fn run(cfg: DuplicateConfig) -> Result<()> {
    ensure!(cfg.concurrency >= 1, "--concurrency must be >= 1");
    ensure!(cfg.deals_per_pair >= 1, "--deals-per-pair must be >= 1");
    ensure!(
        cfg.profiles.len() >= 2,
        "at least 2 distinct profiles are required"
    );
    let _lock = controller_lock(&cfg.root)?;
    std::fs::create_dir_all(cfg.root.join("attempts"))?;

    let sys_env = |key: &str| std::env::var(key).ok();
    let paths = Paths::resolve(Some(&cfg.profiles_config), None, &sys_env);
    let settings_file = paths
        .load()
        .map_err(|e| anyhow::anyhow!("{e}"))?
        .context("loading --profiles-config")?;
    for prof in &cfg.profiles {
        plan(
            None,
            Some(&settings_file),
            &paths,
            Some(prof),
            &Overrides::default(),
        )
        .map_err(|e| anyhow::anyhow!("invalid profile {prof}: {e}"))?
        .with_context(|| format!("profile {prof} resolved to house instead of LLM"))?;
    }

    let decks = load_constructed_decks(&cfg.decks_dir)?;
    ensure!(
        decks.len() >= 2,
        "need at least 2 non-Commander Constructed decks in {}",
        cfg.decks_dir.display()
    );
    let legs = build_schedule(&cfg, &decks);
    eprintln!(
        "[duplicate] {} profiles, {} Constructed decks, {} mirrored deals ({} legs), concurrency={}",
        cfg.profiles.len(),
        decks.len(),
        legs.len() / 2,
        legs.len(),
        cfg.concurrency
    );
    execute_legs(Arc::new(cfg), Arc::new(settings_file), legs).await
}

async fn execute_legs(
    cfg: Arc<DuplicateConfig>,
    settings_file: Arc<SeatSettings>,
    legs: Vec<LegSpec>,
) -> Result<()> {
    let sem = Arc::new(Semaphore::new(cfg.concurrency));
    let halted = Arc::new(AtomicBool::new(false));
    let stop_file = cfg.root.join("STOP");
    let drain_file = cfg.root.join("DRAIN");
    let mut tasks = JoinSet::new();

    for leg in legs {
        while let Some(joined) = tasks.try_join_next() {
            joined.context("duplicate worker task panicked")?;
        }
        if halted.load(Ordering::SeqCst) || stop_file.exists() || drain_file.exists() {
            eprintln!("[duplicate] STOP/DRAIN detected; halting new leg assignments");
            break;
        }
        let attempt_dir = cfg
            .root
            .join("attempts")
            .join(format!("attempt-{:09}", leg.attempt_index));
        if already_admitted(&attempt_dir) {
            eprintln!(
                "[duplicate] SKIP attempt-{:09} ({} leg {} already admitted)",
                leg.attempt_index, leg.deal_id, leg.leg
            );
            continue;
        }
        let permit = Arc::clone(&sem).acquire_owned().await?;
        if halted.load(Ordering::SeqCst) || stop_file.exists() || drain_file.exists() {
            drop(permit);
            break;
        }
        let cfg_cl = Arc::clone(&cfg);
        let file_cl = Arc::clone(&settings_file);
        let halted_cl = Arc::clone(&halted);
        tasks.spawn(async move {
            let _permit = permit;
            if let Err(err) = run_single_leg(&cfg_cl, &file_cl, &leg, &attempt_dir).await {
                eprintln!(
                    "[duplicate] FAILED attempt-{:09} ({} leg {} [{} vs {}]): {err:#}",
                    leg.attempt_index,
                    leg.deal_id,
                    leg.leg,
                    leg.seat_profiles[0],
                    leg.seat_profiles[1]
                );
                if err.to_string().contains("quota_limited") {
                    halted_cl.store(true, Ordering::SeqCst);
                }
            }
        });
    }

    while let Some(joined) = tasks.join_next().await {
        joined.context("duplicate worker task panicked")?;
    }
    Ok(())
}

fn already_admitted(attempt_dir: &Path) -> bool {
    let Ok(raw) = std::fs::read(attempt_dir.join("receipt.json")) else {
        return false;
    };
    serde_json::from_slice::<Value>(&raw)
        .is_ok_and(|receipt| receipt["status"].as_str() == Some("admitted"))
}

pub(crate) fn load_constructed_decks(dir: &Path) -> Result<Vec<EvalDeck>> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
        .with_context(|| format!("reading decks dir {}", dir.display()))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("txt"))
        .collect();
    paths.sort();
    let mut out = Vec::new();
    for path in paths {
        let raw = std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
        let text = std::str::from_utf8(&raw)
            .with_context(|| format!("invalid UTF-8 in {}", path.display()))?;
        let key = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("deck")
            .to_owned();
        let house = HouseDeck::parse_any(&key, text)
            .with_context(|| format!("parsing {}", path.display()))?;
        if !house.deck.commanders.is_empty() {
            continue;
        }
        let list = list_of(&house.deck);
        out.push(EvalDeck {
            key,
            path,
            sha256: hex_sha256(&raw),
            loaded: house.deck,
            list,
        });
    }
    Ok(out)
}

/// Circle-method 1-factorization of `0..k` so every consecutive round touches
/// disjoint profiles, preventing concurrency hot-spots on a single model.
fn round_robin_pairs(k: usize) -> Vec<(usize, usize)> {
    let m = if k.is_multiple_of(2) { k } else { k + 1 };
    if m < 2 {
        return Vec::new();
    }
    let ring = m - 1;
    let mut pairs = Vec::with_capacity(k * (k - 1) / 2);
    for round in 0..ring {
        for slot in 0..(m / 2) {
            let p1 = if slot == 0 {
                0
            } else {
                1 + (round + slot - 1) % ring
            };
            let p2 = 1 + (round + ring - 1 - slot) % ring;
            if p1 < k && p2 < k {
                pairs.push((p1.min(p2), p1.max(p2)));
            }
        }
    }
    pairs
}

pub(crate) const fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

fn build_schedule(cfg: &DuplicateConfig, decks: &[EvalDeck]) -> Vec<LegSpec> {
    let mut deck_pairs = Vec::new();
    for i in 0..decks.len() {
        for j in (i + 1)..decks.len() {
            deck_pairs.push((i, j));
        }
    }
    let model_pairs = round_robin_pairs(cfg.profiles.len());
    let stride = coprime_stride(deck_pairs.len());
    let mut legs = Vec::with_capacity(model_pairs.len() * cfg.deals_per_pair * 2);
    let mut deal_index = 0_usize;

    for _deal_round in 0..cfg.deals_per_pair {
        for &(ia, ib) in &model_pairs {
            let seed = splitmix64(cfg.base_seed.wrapping_add(deal_index as u64));
            let (d0, d1) = deck_pairs[(deal_index * stride + 5) % deck_pairs.len()];
            let deal_id = format!("deal-{deal_index:06}");
            let pa = cfg.profiles[ia].clone();
            let pb = cfg.profiles[ib].clone();
            for (leg, seat_profiles) in [
                (0_u8, [pa.clone(), pb.clone()]),
                (1_u8, [pb.clone(), pa.clone()]),
            ] {
                legs.push(LegSpec {
                    attempt_index: deal_index * 2 + usize::from(leg),
                    deal_index,
                    deal_id: deal_id.clone(),
                    leg,
                    seed,
                    pair_profiles: [pa.clone(), pb.clone()],
                    seat_profiles,
                    decks: [decks[d0].clone(), decks[d1].clone()],
                    swiss_round: None,
                    swiss_table: None,
                });
            }
            deal_index += 1;
        }
    }
    legs
}

pub(crate) const fn coprime_stride(len: usize) -> usize {
    if len <= 1 {
        return 1;
    }
    let mut cand = 17;
    while gcd(cand, len) != 1 {
        cand += 1;
    }
    cand
}

const fn gcd(mut a: usize, mut b: usize) -> usize {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}

async fn run_single_leg(
    cfg: &DuplicateConfig,
    settings_file: &SeatSettings,
    leg: &LegSpec,
    attempt_dir: &Path,
) -> Result<()> {
    let exec = LegExecParams {
        profiles_config: cfg.profiles_config.clone(),
        call_timeout_seconds: cfg.call_timeout_seconds,
        max_actions: cfg.max_actions,
        rules: TableRules::default(),
    };
    run_single_leg_with_rules(&exec, settings_file, leg, attempt_dir).await
}

pub(crate) async fn run_single_leg_with_rules(
    exec: &LegExecParams,
    settings_file: &SeatSettings,
    leg: &LegSpec,
    attempt_dir: &Path,
) -> Result<()> {
    let started = Instant::now();
    let transcripts_dir = attempt_dir.join("transcripts");
    if transcripts_dir.exists() {
        let _ = std::fs::remove_dir_all(&transcripts_dir);
    }
    std::fs::create_dir_all(&transcripts_dir)?;
    let game_id = Uuid::now_v7().to_string();
    eprintln!(
        "[duplicate] START attempt-{:09} ({} leg {} seed={:#018x}: {} [{}] vs {} [{}])",
        leg.attempt_index,
        leg.deal_id,
        leg.leg,
        leg.seed,
        leg.seat_profiles[0],
        leg.decks[0].key,
        leg.seat_profiles[1],
        leg.decks[1].key,
    );

    let built = build_seat_minds(exec, settings_file, &leg.seat_profiles, &transcripts_dir)?;
    let mut table = DuplicateTable::new(
        leg.seed,
        &game_id,
        [
            (&leg.decks[0].loaded, &leg.decks[0].list),
            (&leg.decks[1].loaded, &leg.decks[1].list),
        ],
        built.minds,
        built.declared,
        &transcripts_dir,
        exec.call_timeout_seconds,
        exec.rules,
    )?;

    let play_res = table
        .play(
            exec.max_actions,
            Duration::from_secs(exec.call_timeout_seconds + 5),
        )
        .await;
    table.finish_transcripts();
    std::fs::write(attempt_dir.join("record.bin"), table.session.take_record())?;

    if let Err(err) = play_res {
        write_failed_receipt(
            exec,
            leg,
            attempt_dir,
            &game_id,
            &built.models,
            &err.to_string(),
        )?;
        return Err(err);
    }
    finalize_leg(
        exec,
        leg,
        attempt_dir,
        &game_id,
        &built.models,
        &table,
        started.elapsed(),
    )
}

struct BuiltSeatMinds {
    minds: [Arc<dyn Mind>; 2],
    models: [String; 2],
    declared: [v1::SeatMind; 2],
}

fn build_seat_minds(
    exec: &LegExecParams,
    settings_file: &SeatSettings,
    seat_profiles: &[String; 2],
    transcripts_dir: &Path,
) -> Result<BuiltSeatMinds> {
    let sys_env = |key: &str| std::env::var(key).ok();
    let paths = Paths::resolve(Some(&exec.profiles_config), None, &sys_env);
    let mut minds_vec = Vec::with_capacity(2);
    let mut models_vec = Vec::with_capacity(2);
    let mut declared_vec = Vec::with_capacity(2);

    for profile in seat_profiles {
        let mut seat_plan = plan(
            None,
            Some(settings_file),
            &paths,
            Some(profile),
            &Overrides {
                think_secs: Some(exec.call_timeout_seconds),
                ..Overrides::default()
            },
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?
        .with_context(|| format!("profile {profile} produced no LLM plan"))?;
        seat_plan.settings.transcripts = Some(transcripts_dir.to_path_buf());
        seat_plan.settings.reports = Some(transcripts_dir.to_path_buf());
        let decl = baylee_seat::declare::llm(&seat_plan.settings);
        let model_name = if decl.model.is_empty() {
            seat_plan.settings.model.clone()
        } else {
            decl.model.clone()
        };
        let built = llm::build(&seat_plan, &sys_env).map_err(|e| anyhow::anyhow!("{e}"))?;
        minds_vec.push(built.mind);
        models_vec.push(model_name);
        declared_vec.push(decl);
    }

    let [m0, m1]: [Arc<dyn Mind>; 2] = minds_vec
        .try_into()
        .map_err(|_| anyhow::anyhow!("expected 2 minds"))?;
    let [id0, id1]: [String; 2] = models_vec
        .try_into()
        .map_err(|_| anyhow::anyhow!("expected 2 model ids"))?;
    let [d0, d1]: [v1::SeatMind; 2] = declared_vec
        .try_into()
        .map_err(|_| anyhow::anyhow!("expected 2 declarations"))?;
    Ok(BuiltSeatMinds {
        minds: [m0, m1],
        models: [id0, id1],
        declared: [d0, d1],
    })
}

fn finalize_leg(
    exec: &LegExecParams,
    leg: &LegSpec,
    attempt_dir: &Path,
    game_id: &str,
    models: &[String; 2],
    table: &DuplicateTable,
    elapsed: Duration,
) -> Result<()> {
    let s0 = table.cores[0].stats();
    let s1 = table.cores[1].stats();
    let house_used = s0.answered.house > 0
        || s1.answered.house > 0
        || s0.answered.least > 0
        || s1.answered.least > 0;
    let quota_limited = s0.housed || s1.housed || (exec.rules.strict_zero_house && house_used);
    let both_answered =
        (s0.answered.mind > 0 || s0.asked == 0) && (s1.answered.mind > 0 || s1.asked == 0);
    let admitted =
        !quota_limited && both_answered && (!exec.rules.strict_zero_house || !house_used);
    let (draw, winner_seat, reason) = match table.result {
        Some(GameResult {
            winner: Some(Victor::Player(pid)),
            reason,
        }) => (false, Some(u64::from(pid.get())), format!("{reason:?}")),
        Some(GameResult {
            winner: Some(Victor::Team(team)),
            reason,
        }) => (false, Some(u64::from(team)), format!("{reason:?}")),
        Some(GameResult {
            winner: None,
            reason,
        }) => (true, None, format!("{reason:?}")),
        None => (true, None, "MaxActions".to_owned()),
    };
    let reports_count = read_jsonl(
        &attempt_dir
            .join("transcripts")
            .join(baylee_seat::reports::FILE),
    )
    .map_or(0, |rows| rows.len());
    let receipt = build_receipt(
        exec,
        leg,
        attempt_dir,
        game_id,
        models,
        table,
        admitted,
        quota_limited,
        (draw, winner_seat, &reason),
        reports_count,
    );
    write_atomic_json(&attempt_dir.join("receipt.json"), &receipt)?;

    if !admitted {
        let why = if house_used && exec.rules.strict_zero_house {
            "strict_zero_house: at least one seat fell back to house/least"
        } else if quota_limited {
            "quota_limited: at least one mind ran out of quota and fell back to house"
        } else {
            "mind_never_answered: a seat had 0 mind answers"
        };
        bail!("{why}");
    }
    eprintln!(
        "[duplicate] ADMITTED attempt-{:09} ({} leg {}) in {:.1}s — winner={:?} draw={} actions={} reports={}",
        leg.attempt_index,
        leg.deal_id,
        leg.leg,
        elapsed.as_secs_f64(),
        winner_seat,
        draw,
        table.actions,
        reports_count,
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn build_receipt(
    exec: &LegExecParams,
    leg: &LegSpec,
    attempt_dir: &Path,
    game_id: &str,
    models: &[String; 2],
    table: &DuplicateTable,
    admitted: bool,
    quota_limited: bool,
    terminal: (bool, Option<u64>, &str),
    reports_count: usize,
) -> Value {
    let (draw, winner_seat, reason) = terminal;
    let deck_json = |d: &EvalDeck| {
        json!({
            "format": "Constructed",
            "key": d.key,
            "name": d.loaded.name,
            "path": d.path.display().to_string(),
            "sha256": d.sha256,
            "source": "eval_duplicate",
            "source_file": d.path.display().to_string(),
        })
    };
    let seat_json = |seat: usize| {
        let st = table.cores[seat].stats();
        json!({
            "seat": seat,
            "model": models[seat],
            "turns": st.turns,
            "profile_fallbacks": {
                "played": [],
                "refused": [],
                "took_over": [],
            },
            "transcript_answer_counts": {
                "llm": st.answered.mind,
                "house": st.answered.house,
                "standing": st.answered.standing,
                "other": st.answered.least,
            },
        })
    };
    let mut dup_block = json!({
        "deal_id": leg.deal_id,
        "deal_index": leg.deal_index,
        "leg": leg.leg,
        "seed": leg.seed,
        "pair": leg.pair_profiles,
        "decks": [leg.decks[0].key, leg.decks[1].key],
    });
    if let (Some(round), Some(tbl)) = (leg.swiss_round, leg.swiss_table) {
        dup_block["swiss"] = json!({ "round": round, "table": tbl });
    }
    json!({
        "status": if admitted { "admitted" } else { "failed" },
        "admission": "Duplicate-seed in-process evaluation",
        "duplicate": dup_block,
        "request": {
            "attempt_id": format!("attempt-{:09}", leg.attempt_index),
            "bc_only": false,
            "numbers_only": true,
            "call_timeout_seconds": exec.call_timeout_seconds,
            "declared_deck_format": "Constructed",
            "profiles": leg.seat_profiles,
            "models": models,
            "profiles_config": exec.profiles_config.display().to_string(),
            "decks": [deck_json(&leg.decks[0]), deck_json(&leg.decks[1])],
            "transcriptsdir": attempt_dir.join("transcripts").display().to_string(),
        },
        "technical": {
            "status": if admitted { "complete" } else { "failed" },
            "game_id": game_id,
            "actual_seed": leg.seed,
            "actions": table.actions,
            "llm_reports": reports_count,
            "quota_limited": quota_limited,
            "terminal": {
                "draw": draw,
                "winner_seat": winner_seat,
                "reason": reason,
            },
            "seats": [seat_json(0), seat_json(1)],
        },
    })
}

fn write_failed_receipt(
    exec: &LegExecParams,
    leg: &LegSpec,
    attempt_dir: &Path,
    game_id: &str,
    models: &[String; 2],
    error: &str,
) -> Result<()> {
    let mut dup_block = json!({
        "deal_id": leg.deal_id,
        "deal_index": leg.deal_index,
        "leg": leg.leg,
        "seed": leg.seed,
        "pair": leg.pair_profiles,
    });
    if let (Some(round), Some(tbl)) = (leg.swiss_round, leg.swiss_table) {
        dup_block["swiss"] = json!({ "round": round, "table": tbl });
    }
    let receipt = json!({
        "status": "failed",
        "failure_reason": error,
        "duplicate": dup_block,
        "request": {
            "attempt_id": format!("attempt-{:09}", leg.attempt_index),
            "numbers_only": true,
            "call_timeout_seconds": exec.call_timeout_seconds,
            "profiles": leg.seat_profiles,
            "models": models,
        },
        "technical": {
            "status": "failed",
            "game_id": game_id,
            "actual_seed": leg.seed,
        },
    });
    write_atomic_json(&attempt_dir.join("receipt.json"), &receipt)
}

struct DuplicateTable {
    game_id: String,
    rules: TableRules,
    session: Session,
    cores: Vec<SeatCore>,
    minds: Vec<Arc<dyn Mind>>,
    transcripts: Vec<Transcript>,
    inbox: VecDeque<(PlayerId, Envelope)>,
    outbox: VecDeque<(PlayerId, PlayerAction)>,
    asks: Vec<(usize, Box<Request>)>,
    result: Option<GameResult>,
    actions: u32,
}

impl DuplicateTable {
    #[allow(clippy::too_many_arguments)]
    fn new(
        seed: u64,
        game_id: &str,
        decks: [(&LoadedDeck, &DeckList); 2],
        minds: [Arc<dyn Mind>; 2],
        declared: [v1::SeatMind; 2],
        transcripts_dir: &Path,
        call_timeout_secs: u64,
        rules: TableRules,
    ) -> Result<Self> {
        let mut preset = preset_for(seed, decks[0].0, decks[1].0);
        if rules.tournament_constructed {
            preset.house_rules = baylee_core::preset::HouseRules {
                mulligan_free_first: false,
                free_mulligans: Some(0),
                auto_discard_empty_optional_triggers: false,
                ..baylee_core::preset::HouseRules::for_a_table()
            };
        }
        for seat in &mut preset.seats {
            seat.controller = SeatController::Open;
        }
        let mut session = Session::new_recorded(&preset, env!("CARGO_PKG_VERSION"))
            .context("Session::new_recorded returned None")?;
        session.set_fair_scouting(true);
        let names = ["P1", "P2"]
            .iter()
            .zip(&minds)
            .map(|(tag, mind)| seat_name(mind.disclosure(), tag))
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| anyhow::anyhow!("seat_name: {e}"))?;
        session.describe(game_id.to_owned(), names);

        let bridge_cfg = BridgeConfig {
            think: Duration::from_secs(call_timeout_secs),
            margin: Duration::ZERO,
            retries: if rules.strict_zero_house { 3 } else { 1 },
            down_after: if rules.strict_zero_house { 1 } else { 8 },
            ..BridgeConfig::default()
        };
        let cores = decks
            .iter()
            .zip(&minds)
            .zip(declared)
            .map(|(((_, list), mind), decl)| {
                SeatCore::new(bridge_cfg.clone(), (*list).clone(), mind.disclosure())
                    .with_mind(decl)
            })
            .collect();
        let transcripts = (0..2)
            .map(|seat| {
                Transcript::file(&transcripts_dir.join(format!("{game_id}-seat{seat}.jsonl")))
            })
            .collect::<std::io::Result<Vec<_>>>()?;
        let mut inbox = VecDeque::new();
        for seat in 0..2 {
            let pid = PlayerId::new(seat);
            inbox.push_back((pid, session.game_static_envelope(pid)));
        }
        inbox.extend(session.pump());

        Ok(Self {
            game_id: game_id.to_owned(),
            rules,
            session,
            cores,
            minds: minds.into(),
            transcripts,
            inbox,
            outbox: VecDeque::new(),
            asks: Vec::new(),
            result: None,
            actions: 0,
        })
    }

    async fn play(&mut self, max_actions: u32, hard_timeout: Duration) -> Result<()> {
        while self.result.is_none() && self.actions < max_actions {
            let stepped = self.step(hard_timeout).await?;
            if !stepped && self.result.is_none() {
                bail!(
                    "duplicate table stalled after {} actions (pending {:?})",
                    self.actions,
                    self.session.pending()
                );
            }
        }
        self.deliver()?;
        Ok(())
    }

    async fn step(&mut self, hard_timeout: Duration) -> Result<bool> {
        self.deliver()?;
        if !self.asks.is_empty() {
            self.think(hard_timeout).await?;
            return Ok(true);
        }
        let Some((seat, action)) = self.outbox.pop_front() else {
            return Ok(false);
        };
        self.actions += 1;
        match self.session.act(seat, action) {
            Ok(routed) => self.inbox.extend(routed),
            Err(reason) => {
                self.inbox.push_back((seat, error_envelope(&reason)));
                self.inbox
                    .extend(self.session.reask(seat).into_iter().map(|env| (seat, env)));
            }
        }
        Ok(true)
    }

    fn deliver(&mut self) -> Result<()> {
        while let Some((seat, envelope)) = self.inbox.pop_front() {
            let at = usize::from(seat.get());
            let steps = self.cores[at].hear(&envelope.encode_to_vec());
            self.take_steps(at, steps)?;
        }
        Ok(())
    }

    async fn think(&mut self, hard_timeout: Duration) -> Result<()> {
        let asks = std::mem::take(&mut self.asks);
        let futures: Vec<_> = asks
            .into_iter()
            .map(|(seat, request)| {
                let question = request.question;
                let fut = self.minds[seat].decide(*request);
                async move {
                    let res = tokio::time::timeout(hard_timeout, fut).await;
                    (seat, question, res)
                }
            })
            .collect();
        for (seat, question, outcome) in join_all(futures).await {
            let steps = match outcome {
                Ok(ans) => self.cores[seat].answered(question, ans),
                Err(_) => self.cores[seat].expired(question),
            };
            self.take_steps(seat, steps)?;
        }
        Ok(())
    }

    fn take_steps(&mut self, seat: usize, steps: Vec<Step>) -> Result<()> {
        for step in steps {
            match step {
                Step::Send(_) | Step::MindDown => {}
                Step::Answer { action, by, .. } => {
                    if self.rules.strict_zero_house
                        && matches!(by, baylee_seat::By::House | baylee_seat::By::Least)
                    {
                        for note in self.cores[seat].take_notes() {
                            self.transcripts[seat].write(&note);
                        }
                        let st = self.cores[seat].stats();
                        bail!(
                            "strict_zero_house: seat {seat} fell back to {by:?} (fallbacks={:?}, housed={})",
                            st.fallbacks,
                            st.housed
                        );
                    }
                    let pid = PlayerId::new(u8::try_from(seat).expect("2 seats"));
                    self.outbox.push_back((pid, action));
                }
                Step::Ask(req) => self.asks.push((seat, req)),
                Step::Cancel(q) => self.asks.retain(|(s, r)| *s != seat || r.question != q),
                Step::Leave(reason) => bail!("seat {seat} left the table: {reason}"),
                Step::Over(res) => self.result = Some(res),
            }
        }
        for note in self.cores[seat].take_notes() {
            self.transcripts[seat].write(&note);
        }
        Ok(())
    }

    fn finish_transcripts(&mut self) {
        for (seat, core) in self.cores.iter_mut().enumerate() {
            for note in core.take_notes() {
                self.transcripts[seat].write(&note);
            }
            self.transcripts[seat].write_value(&json!({ "summary": core.stats() }));
            self.transcripts[seat].flush();
            if let Ok(seat_u8) = u8::try_from(seat) {
                self.minds[seat].end_game(&self.game_id, seat_u8);
            }
        }
    }
}

fn error_envelope(message: &str) -> Envelope {
    Envelope {
        msg: Some(v1::envelope::Msg::Error(v1::Error {
            code: 1,
            message: message.to_owned(),
        })),
    }
}

fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().fold(String::with_capacity(64), |mut s, b| {
        use std::fmt::Write as _;
        let _ = write!(s, "{b:02x}");
        s
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::tests::TestDir;
    use baylee_ai::AIProfile;
    use baylee_seat::HouseMind;
    use std::collections::BTreeSet;

    #[test]
    fn round_robin_covers_every_unordered_pair_in_disjoint_rounds() {
        let pairs = round_robin_pairs(10);
        assert_eq!(pairs.len(), 45);
        let unique: BTreeSet<(usize, usize)> = pairs.iter().copied().collect();
        assert_eq!(unique.len(), 45);
        for round in 0..9 {
            let chunk = &pairs[round * 5..(round + 1) * 5];
            let mut seen = BTreeSet::new();
            for &(a, b) in chunk {
                assert!(a < b);
                assert!(seen.insert(a));
                assert!(seen.insert(b));
            }
            assert_eq!(seen.len(), 10);
        }
    }

    #[tokio::test]
    async fn mirrored_legs_deal_identical_opening_hands_and_seat_roles() {
        let temp = TestDir::new("duplicate-mirror");
        let eval_dir = baylee_train::working::repo_root().join("data/decks/eval");
        let eval_decks = load_constructed_decks(&eval_dir).expect("eval decks load");
        assert_eq!(eval_decks.len(), 12, "expected 12 Constructed eval decks");
        let d0 = &eval_decks[0];
        let d1 = &eval_decks[1];
        let seed = 0x2026_1113_cafe_babe_u64;

        let mut hands = Vec::new();
        for leg in 0..2 {
            let dir = temp.path().join(format!("leg-{leg}"));
            std::fs::create_dir_all(&dir).unwrap();
            let m_a: Arc<dyn Mind> = Arc::new(HouseMind::new(AIProfile::default()));
            let m_b: Arc<dyn Mind> = Arc::new(HouseMind::new(AIProfile::default()));
            let minds = if leg == 0 { [m_a, m_b] } else { [m_b, m_a] };
            let decl = [
                baylee_seat::declare::house("steady"),
                baylee_seat::declare::house("sharp"),
            ];
            let mut table = DuplicateTable::new(
                seed,
                &format!("mirror-{leg}"),
                [(&d0.loaded, &d0.list), (&d1.loaded, &d1.list)],
                minds,
                decl,
                &dir,
                30,
                TableRules::default(),
            )
            .unwrap();
            let v0 = table.session.observe_view(PlayerId::new(0));
            let v1 = table.session.observe_view(PlayerId::new(1));
            let h0: Vec<_> = v0.hand.iter().map(|c| c.name.clone()).collect();
            let h1: Vec<_> = v1.hand.iter().map(|c| c.name.clone()).collect();
            assert_eq!(h0.len(), 7);
            assert_eq!(h1.len(), 7);
            hands.push((h0, h1));
            table.play(1500, Duration::from_secs(10)).await.unwrap();
            assert!(table.result.is_some());
        }
        assert_eq!(
            hands[0].0, hands[1].0,
            "Seat 0 opening hand must match byte-for-byte"
        );
        assert_eq!(
            hands[0].1, hands[1].1,
            "Seat 1 opening hand must match byte-for-byte"
        );
    }
}
