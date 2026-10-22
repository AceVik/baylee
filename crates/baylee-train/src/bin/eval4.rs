//! Multi-format duplicate evaluator and multi-seat RL rollout generator for
//! Playernet v4 (`NetPlayer4`).
//!
//! Supports:
//! - Any table size `2..=8` (`--seats`) and team configuration (`--teams`);
//! - Duplicate deal evaluation: compares `NetPlayer4` on Side A against a
//!   matched `House` baseline on Side A with the exact same deal seed and decks;
//! - Dynamic latent thinking depth `--thinking-steps` (`K in {1, 2, 4, 8}`);
//! - Zero-shot unseen card evaluation (`--held-out-cards`), generating decks
//!   that include the 5% held-out card pool excluded from training runs;
//! - Multi-seat RL rollout recording (`--record-out <dir>`), writing `run.json`,
//!   `games.jsonl`, and `records/*.jsonl.gz` ready for `convert4` and `train_rl4.py`.

use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::{BufWriter, Write as _};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use anyhow::{Context as _, bail};
use baylee_cards::formats::Shape;
use baylee_core::ids::PlayerId;
use baylee_core::preset::AIProfile;
use baylee_engine::choice::Pending;
use baylee_gamehost::Session;
use baylee_train::convert4::RolloutStepSidecar4;
use baylee_train::deckgen::{self, Archetype};
use baylee_train::features4::{
    CoverageMatrix4, SourceKind4, StepProvenance4, side_utility, view_only_state_hash,
};
use baylee_train::housedeck::HouseDeck;
use baylee_train::netplay4::NetPlayer4;
use baylee_train::selfplay::table_for;
use baylee_train::working::{Working, repo_root};
use clap::Parser;
use flate2::Compression;
use flate2::write::GzEncoder;
use serde_json::{Value, json};

#[derive(Parser, Debug)]
#[command(about = "Multi-format duplicate evaluator and RL rollout generator for Playernet v4")]
struct Args {
    /// Path to Playernet v4 `model.onnx` or checkpoint directory.
    #[arg(long)]
    model: PathBuf,
    /// Number of deals to play (each deal plays Net + optional House baseline).
    #[arg(long, default_value_t = 40)]
    deals: u64,
    /// Seats at the table (`2..=8`).
    #[arg(long, default_value_t = 2)]
    seats: usize,
    /// Team assignment per seat (e.g. `1,2,1,2` for 2v2); empty = FFA.
    #[arg(long, value_delimiter = ',')]
    teams: Vec<u8>,
    /// Opponent house profile (`novice`, `casual`, `steady`, `sharp`, `expert`).
    #[arg(long, default_value = "steady")]
    against: String,
    /// Baseline house profile for duplicate comparison (`steady`, `sharp`, `expert`).
    #[arg(long, default_value = "steady")]
    baseline: String,
    /// Latent thinking steps `K` (`1..=8`).
    #[arg(long, default_value_t = 2)]
    thinking_steps: u8,
    /// Sampling temperature `tau` (`0.0` = greedy argmax).
    #[arg(long, default_value_t = 0.0)]
    temperature: f32,
    /// Number of generated decks to build (`0` = use `--decks-manifest` only).
    #[arg(long, default_value_t = 32)]
    generated: usize,
    /// Shape for generated/manifest decks (`constructed`, `highlander`, `commander`).
    #[arg(long, default_value = "constructed")]
    shape: String,
    /// Optional path to `data/decks/manifest.v4.json` to load real-world decks.
    #[arg(long)]
    decks_manifest: Option<PathBuf>,
    /// Split to filter when `--decks-manifest` is given (`train`, `val`, `test`).
    #[arg(long, default_value = "test")]
    deck_split: String,
    /// Optional `holdout_tier` filter when `--decks-manifest` is given
    /// (`eval_benchmark`, `unseen_deck_known_cards`, `unseen_cards`).
    #[arg(long, default_value = "")]
    holdout_tier: String,
    /// Build/filter decks including the held-out cards excluded from training.
    #[arg(long)]
    held_out_cards: bool,
    /// Clamp AI scouting so House opponents have no omniscient hand/deck peek.
    #[arg(long, default_value_t = true)]
    fair_scouting: bool,
    /// Skip running the House baseline duplicate game (useful for pure RL rollouts).
    #[arg(long)]
    no_baseline: bool,
    /// Optional output directory for recorded games (`run.json`, `games.jsonl`, `records/`).
    #[arg(long)]
    record_out: Option<PathBuf>,
    /// Output JSON summary file path.
    #[arg(long)]
    out: PathBuf,
    /// Worker threads (`0` = all cores).
    #[arg(long, default_value_t = 0)]
    threads: usize,
    /// First deal seed (shuffle/opening hand RNG).
    #[arg(long, default_value_t = 10_000)]
    first_seed: u64,
    /// Independent policy sampling seed (never mixed with deal `seed`, `P0-7`).
    #[arg(long, default_value_t = 0x517c_c1b7_2722_0a95)]
    policy_seed: u64,
    /// Max decisions per two seats.
    #[arg(long, default_value_t = 5000)]
    max_decisions: u64,
}

#[derive(Default)]
struct EvalStats {
    deals_completed: u64,
    net_utility_sum: f64,
    base_utility_sum: f64,
    net_wins: u64,
    base_wins: u64,
    net_draws: u64,
    base_draws: u64,
    net_answer_caps: u64,
    base_answer_caps: u64,
    net_answers: u64,
    house_fallbacks: u64,
    refused: u64,
    stalls: u64,
    latencies_ms: Vec<f64>,
    coverage: CoverageMatrix4,
}

fn parse_profile(name: &str) -> anyhow::Result<AIProfile> {
    AIProfile::named(name).with_context(|| format!("unknown AIProfile {name}"))
}

fn is_net_seat(seat: usize, focal_seat: usize, teams: &[Option<u8>]) -> bool {
    if seat == focal_seat {
        return true;
    }
    if let Some(Some(tf)) = teams.get(focal_seat)
        && let Some(Some(ts)) = teams.get(seat)
    {
        return tf == ts;
    }
    false
}

#[allow(clippy::too_many_arguments)]
fn play_one_table(
    game_id: &str,
    deal_idx: u64,
    focal_seat: usize,
    seed: u64,
    policy_seed: u64,
    decks: &[&HouseDeck],
    profiles: &[AIProfile],
    teams_u8: &[u8],
    teams: &[Option<u8>],
    mut net: Option<&mut NetPlayer4>,
    fair_scouting: bool,
    max_decisions: u64,
    record_steps: bool,
) -> anyhow::Result<(
    f32,
    bool,
    u64,
    u64,
    u64,
    u64,
    Vec<f64>,
    Vec<u8>,
    Value,
    Vec<RolloutStepSidecar4>,
)> {
    let preset = table_for(seed, decks, profiles, teams_u8);
    let n_seats = decks.len();
    if let Some(n) = net.as_deref_mut() {
        // Independent policy RNG seed: never depends on deal shuffle `seed` (P0-7)
        let indep_policy_seed = policy_seed
            ^ deal_idx.wrapping_mul(0x9e37_79b9_7f4a_7c15)
            ^ ((n_seats as u64) << 48);
        n.reseed(indep_policy_seed);
        n.init_game(&preset);
    }

    let mut session = if record_steps {
        Session::new_recorded(&preset, baylee_build::short()).context("new_recorded")?
    } else {
        Session::new(&preset).context("Session::new")?
    };
    session.describe(
        game_id.to_owned(),
        (0..n_seats).map(|s| s.to_string()).collect(),
    );
    session.set_fair_scouting(fair_scouting);
    if net.is_some() {
        for s in 0..n_seats {
            if is_net_seat(s, focal_seat, teams) {
                session.take_over(PlayerId::new(s as u8));
            }
        }
    }

    let mut net_answers = 0_u64;
    let mut fallbacks = 0_u64;
    let mut refused = 0_u64;
    let mut stalls = 0_u64;
    let mut latencies = Vec::new();
    let mut rollout_sidecar: Vec<RolloutStepSidecar4> = Vec::new();
    let mut visit_counts = vec![std::collections::BTreeMap::<u64, u8>::new(); n_seats];

    loop {
        if let Pending::GameOver(result) = session.pending() {
            let winners: Vec<u8> = session
                .winning_seats(*result)
                .into_iter()
                .map(|w| w.get())
                .collect();
            let surviving: Vec<u8> = if winners.is_empty() {
                (0..n_seats as u8).collect()
            } else {
                winners.clone()
            };
            let util = side_utility(focal_seat as u8, teams, &surviving, &winners);
            let outcome_json = match winners.as_slice() {
                [] => json!({"kind": "draw"}),
                [w] => json!({"kind": "won", "seat": w}),
                ws => json!({"kind": "team_won", "seats": ws}),
            };
            return Ok((
                util,
                false,
                net_answers,
                fallbacks,
                refused,
                stalls,
                latencies,
                session.take_record(),
                outcome_json,
                rollout_sidecar,
            ));
        }
        if session.decision_seq() >= max_decisions {
            return Ok((
                0.5,
                true,
                net_answers,
                fallbacks,
                refused,
                stalls,
                latencies,
                session.take_record(),
                json!({"kind": "answer_cap"}),
                rollout_sidecar,
            ));
        }
        session.pump_at_most(32);

        // Passively observe the table for all neural seats after opponent progression (P0-4)
        if let Some(n) = net.as_deref_mut() {
            for s in 0..n_seats {
                if is_net_seat(s, focal_seat, teams) {
                    let pid = PlayerId::new(s as u8);
                    if session.view_for(pid).is_none() {
                        let obs_v = session.observe_view(pid);
                        n.observe(&obs_v);
                    }
                }
            }
        }

        let mut progressed = false;
        for s in 0..n_seats {
            if !is_net_seat(s, focal_seat, teams) {
                continue;
            }
            let pid = PlayerId::new(s as u8);
            let Some((pending, view)) = session.view_for(pid) else {
                continue;
            };
            progressed = true;
            let vh = view_only_state_hash(&view, &pending);
            let vc = visit_counts[s].entry(vh).or_insert(0);
            *vc = vc.saturating_add(1);
            if *vc > 4 {
                stalls += 1;
                fallbacks += 1;
                let act = session.house_action(pid).context("house fallback")?;
                if session.act(pid, act).is_ok() && record_steps {
                    rollout_sidecar.push(RolloutStepSidecar4 {
                        provenance: StepProvenance4 {
                            source: SourceKind4::House,
                            has_logprob: false,
                            ..StepProvenance4::default()
                        },
                        free_mask: Vec::new(),
                        pick_sequence: Vec::new(),
                    });
                }
                continue;
            }

            if let Some(n) = net.as_deref_mut() {
                let t0 = Instant::now();
                let ans = n.act_detailed(&view, &pending, teams);
                let dt_ms = t0.elapsed().as_secs_f64() * 1000.0;
                if ans.provenance.source == SourceKind4::Net {
                    latencies.push(dt_ms);
                    net_answers += 1;
                } else if ans.provenance.source == SourceKind4::House {
                    fallbacks += 1;
                }
                if session.act(pid, ans.action).is_ok() {
                    if record_steps {
                        rollout_sidecar.push(RolloutStepSidecar4 {
                            provenance: ans.provenance,
                            free_mask: ans.target.free_mask,
                            pick_sequence: ans.target.pick_sequence,
                        });
                    }
                } else {
                    refused += 1;
                    let fb = session.house_action(pid).context("house fallback after refusal")?;
                    if session.act(pid, fb).is_ok() && record_steps {
                        rollout_sidecar.push(RolloutStepSidecar4 {
                            provenance: StepProvenance4 {
                                source: SourceKind4::House,
                                has_logprob: false,
                                ..StepProvenance4::default()
                            },
                            free_mask: Vec::new(),
                            pick_sequence: Vec::new(),
                        });
                    }
                }
            }
        }
        if !progressed && net.is_none() {
            // Pure house baseline progresses via pump_at_most(32)
            continue;
        }
    }
}

fn load_manifest_decks(
    manifest_path: &PathBuf,
    split: &str,
    shape_str: &str,
    holdout_tier: &str,
    held_out_cards: bool,
) -> anyhow::Result<Vec<HouseDeck>> {
    let raw = fs::read(manifest_path)
        .with_context(|| format!("reading manifest {}", manifest_path.display()))?;
    let val: Value = serde_json::from_slice(&raw)?;
    let root = repo_root();
    let mut out = Vec::new();
    if let Some(entries) = val["decks"].as_array() {
        for entry in entries {
            let entry_split = entry["split"].as_str().unwrap_or("");
            let entry_fmt = entry["format"].as_str().unwrap_or("");
            let entry_tier = entry["holdout_tier"].as_str().unwrap_or("");
            if !split.is_empty() && split != "all" && entry_split != split {
                continue;
            }
            if !shape_str.is_empty() && entry_fmt != shape_str {
                continue;
            }
            if !holdout_tier.is_empty() {
                if entry_tier != holdout_tier {
                    continue;
                }
            } else if split == "test" {
                if held_out_cards && entry_tier != "unseen_cards" {
                    continue;
                }
                if !held_out_cards && entry_tier == "unseen_cards" {
                    continue;
                }
            }
            let Some(rel_path) = entry["path"].as_str() else {
                continue;
            };
            let full_path = root.join(rel_path);
            let Ok(text) = fs::read_to_string(&full_path) else {
                continue;
            };
            let key = entry["id"]
                .as_str()
                .unwrap_or(rel_path)
                .to_owned();
            let Ok(hd) = HouseDeck::parse_any(&key, &text) else {
                continue;
            };
            out.push(hd);
        }
    }
    Ok(out)
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    if !(2..=8).contains(&args.seats) {
        bail!("--seats must be between 2 and 8");
    }
    let teams: Vec<Option<u8>> = if args.teams.is_empty() {
        vec![None; args.seats]
    } else {
        if args.teams.len() != args.seats {
            bail!("--teams length must equal --seats");
        }
        args.teams
            .iter()
            .map(|&t| (t != 0).then_some(t))
            .collect()
    };

    let opp_profile = parse_profile(&args.against)?;
    let base_profile = parse_profile(&args.baseline)?;

    let working = Working::scan(&repo_root())?;
    let shape = match args.shape.as_str() {
        "constructed" => Shape::Constructed,
        "highlander" => Shape::Highlander,
        "commander" => Shape::Commander,
        other => bail!("unknown shape {other}"),
    };

    let held = deckgen::held_out(&working, 20);
    let empty_held = BTreeSet::new();
    let pool = if args.held_out_cards {
        // Include all working cards (including the held-out 5%) and seed from the held-out region
        deckgen::Pool::new(&working, &empty_held)
    } else {
        deckgen::Pool::new(&working, &held)
    };

    let mut decks: Vec<HouseDeck> = Vec::new();
    if let Some(manifest_path) = &args.decks_manifest {
        decks.extend(load_manifest_decks(
            manifest_path,
            &args.deck_split,
            &args.shape,
            &args.holdout_tier,
            args.held_out_cards,
        )?);
    }
    let mut attempt = 0_u64;
    let seed_base = if args.held_out_cards { 99_999_u64 } else { 7_777_u64 };
    let target_decks = if args.decks_manifest.is_some() && args.generated == 0 {
        decks.len().max(args.seats)
    } else {
        decks.len() + args.generated.max(args.seats * 2)
    };
    while decks.len() < target_decks && attempt < 2000 {
        let arch = Archetype::ALL[decks.len() % Archetype::ALL.len()];
        let s = seed_base.wrapping_mul(1_000_003).wrapping_add(attempt);
        attempt += 1;
        let Ok(g) = deckgen::generate(&pool, shape, arch, s) else {
            continue;
        };
        if args.held_out_cards {
            // Require at least 1 held-out card in the generated deck
            let has_held = g.main.iter().chain(&g.leaders).any(|c| held.contains(c));
            if !has_held {
                continue;
            }
        }
        decks.push(HouseDeck {
            key: format!("eval-d{:03}", decks.len()),
            deck: g.loaded(),
        });
    }
    if decks.len() < args.seats {
        bail!("could not load/generate enough decks ({} < {})", decks.len(), args.seats);
    }

    if let Some(rec_dir) = &args.record_out {
        fs::create_dir_all(rec_dir.join("records"))?;
        let deck_json: Vec<_> = decks
            .iter()
            .map(|d| {
                let cards: Vec<u32> = d
                    .deck
                    .main
                    .iter()
                    .chain(&d.deck.commanders)
                    .map(|c| c.index.get())
                    .collect();
                json!({"key": d.key, "cards": cards})
            })
            .collect();
        fs::write(
            rec_dir.join("run.json"),
            serde_json::to_vec_pretty(&json!({
                "run": "eval4-rollout",
                "record_version": baylee_gamehost::record::RECORD_VERSION,
                "training_data": true,
                "decks": deck_json,
                "seats": args.seats,
            }))?,
        )?;
    }

    let threads = match args.threads {
        0 => std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get),
        n => n,
    };
    let max_decisions = args.max_decisions * (args.seats.max(2) as u64 / 2);

    let decks_arc = Arc::new(decks);
    let teams_u8_arc = Arc::new(args.teams.clone());
    let teams_arc = Arc::new(teams);
    let next_deal = Arc::new(AtomicU64::new(0));
    let stats_arc = Arc::new(Mutex::new(EvalStats::default()));
    let games_lines = Arc::new(Mutex::new(Vec::<String>::new()));

    let started = Instant::now();
    let mut handles = Vec::new();
    for worker_id in 0..threads {
        let decks_arc = decks_arc.clone();
        let teams_u8_arc = teams_u8_arc.clone();
        let teams_arc = teams_arc.clone();
        let next_deal = next_deal.clone();
        let stats_arc = stats_arc.clone();
        let games_lines = games_lines.clone();
        let model_path = args.model.clone();
        let rec_out = args.record_out.clone();
        let deals = args.deals;
        let seats = args.seats;
        let first_seed = args.first_seed;
        let policy_seed = args.policy_seed;
        let k_steps = args.thinking_steps;
        let temp = args.temperature;
        let fair = args.fair_scouting;
        let no_base = args.no_baseline;

        handles.push(std::thread::spawn(move || -> anyhow::Result<()> {
            let mut net = NetPlayer4::load(&model_path)?
                .with_thinking_steps(k_steps)
                .with_temperature(temp);

            let shard_name = format!("w{worker_id:02}-0000.jsonl.gz");
            let mut shard_file = match &rec_out {
                Some(dir) => Some((
                    BufWriter::new(File::create(dir.join("records").join(&shard_name))?),
                    0_u64,
                )),
                None => None,
            };

            loop {
                let d = next_deal.fetch_add(1, Ordering::Relaxed);
                if d >= deals {
                    break;
                }
                let table_idx = (d as usize) / seats;
                let focal_seat = (d as usize) % seats;
                let seed = first_seed + (table_idx as u64);
                let mut table_decks = Vec::with_capacity(seats);
                for s in 0..seats {
                    let idx = (table_idx * seats + s) % decks_arc.len();
                    table_decks.push(&decks_arc[idx]);
                }
                let mut profiles = vec![opp_profile; seats];
                for s in 0..seats {
                    if is_net_seat(s, focal_seat, &teams_arc) {
                        profiles[s] = base_profile;
                    }
                }

                let deal_game_id = format!("eval4-deal-{table_idx:05}");
                let rec_game_id = format!("eval4-{d:05}");
                let (
                    net_u,
                    net_capped,
                    n_ans,
                    fb,
                    ref_cnt,
                    st_cnt,
                    lats,
                    rec_bytes,
                    outcome_json,
                    rollout_sidecar,
                ) = play_one_table(
                    &deal_game_id,
                    d,
                    focal_seat,
                    seed,
                    policy_seed,
                    &table_decks,
                    &profiles,
                    &teams_u8_arc,
                    &teams_arc,
                    Some(&mut net),
                    fair,
                    max_decisions,
                    shard_file.is_some(),
                )?;

                if let Some((f, offset)) = &mut shard_file {
                    let mut gz = GzEncoder::new(Vec::new(), Compression::fast());
                    gz.write_all(&rec_bytes)?;
                    let compressed = gz.finish()?;
                    let len = compressed.len() as u64;
                    f.write_all(&compressed)?;
                    f.flush()?;
                    let line = serde_json::to_string(&json!({
                        "game": rec_game_id,
                        "i": d,
                        "seed": seed,
                        "policy_seed": policy_seed,
                        "outcome": outcome_json,
                        "shard": shard_name,
                        "offset": *offset,
                        "len": len,
                        "rollout_sidecar": rollout_sidecar,
                    }))?;
                    *offset += len;
                    games_lines.lock().expect("games lock").push(line);
                }

                let (base_u, base_capped) = if no_base {
                    (0.0, false)
                } else {
                    let (bu, bc, _, _, _, _, _, _, _, _) = play_one_table(
                        &deal_game_id,
                        d,
                        focal_seat,
                        seed,
                        policy_seed,
                        &table_decks,
                        &profiles,
                        &teams_u8_arc,
                        &teams_arc,
                        None,
                        fair,
                        max_decisions,
                        false,
                    )?;
                    (bu, bc)
                };

                let mut st = stats_arc.lock().expect("stats lock");
                st.deals_completed += 1;
                st.net_utility_sum += f64::from(net_u);
                st.base_utility_sum += f64::from(base_u);
                if net_capped {
                    st.net_answer_caps += 1;
                } else if net_u >= 0.99 {
                    st.net_wins += 1;
                } else if net_u > 0.01 {
                    st.net_draws += 1;
                }
                if !no_base {
                    if base_capped {
                        st.base_answer_caps += 1;
                    } else if base_u >= 0.99 {
                        st.base_wins += 1;
                    } else if base_u > 0.01 {
                        st.base_draws += 1;
                    }
                }
                st.net_answers += n_ans;
                st.house_fallbacks += fb;
                st.refused += ref_cnt;
                st.stalls += st_cnt;
                st.latencies_ms.extend(lats);
            }

            let mut st = stats_arc.lock().expect("stats lock");
            for pk in 0..net.coverage.counts.len() {
                for sk in 0..4 {
                    st.coverage.counts[pk][sk] += net.coverage.counts[pk][sk];
                }
            }
            Ok(())
        }));
    }

    for h in handles {
        h.join().expect("worker panicked")?;
    }

    if let Some(rec_dir) = &args.record_out {
        let lines = games_lines.lock().expect("games lock");
        let mut f = BufWriter::new(File::create(rec_dir.join("games.jsonl"))?);
        for l in lines.iter() {
            f.write_all(l.as_bytes())?;
            f.write_all(b"\n")?;
        }
        f.flush()?;
    }

    let mut st = stats_arc.lock().expect("stats lock");
    st.latencies_ms.sort_by(|a, b| a.total_cmp(b));
    let n_lat = st.latencies_ms.len();
    let pct = |q: f64| -> f64 {
        if n_lat == 0 {
            0.0
        } else {
            let idx = ((n_lat as f64 - 1.0) * q).round() as usize;
            st.latencies_ms[idx.min(n_lat - 1)]
        }
    };
    let mean_ms = if n_lat == 0 {
        0.0
    } else {
        st.latencies_ms.iter().sum::<f64>() / (n_lat as f64)
    };
    let deals_f = (st.deals_completed.max(1)) as f64;
    let net_u_mean = st.net_utility_sum / deals_f;
    let base_u_mean = if args.no_baseline {
        None
    } else {
        Some(st.base_utility_sum / deals_f)
    };
    let delta = base_u_mean.map(|bu| net_u_mean - bu);
    let net_win_rate = (st.net_wins as f64) / deals_f;
    let base_win_rate = if args.no_baseline {
        None
    } else {
        Some((st.base_wins as f64) / deals_f)
    };
    let net_losses = st
        .deals_completed
        .saturating_sub(st.net_wins + st.net_draws + st.net_answer_caps);
    let base_losses = if args.no_baseline {
        None
    } else {
        Some(
            st.deals_completed
                .saturating_sub(st.base_wins + st.base_draws + st.base_answer_caps),
        )
    };
    let fb_rate = (st.coverage.fallback_decisions() as f64)
        / ((st.coverage.neural_decisions() + st.coverage.fallback_decisions()).max(1) as f64);

    let summary = json!({
        "model": args.model,
        "seats": args.seats,
        "teams": args.teams,
        "shape": args.shape,
        "decks_manifest": args.decks_manifest,
        "deck_split": args.deck_split,
        "held_out_cards": args.held_out_cards,
        "against": args.against,
        "baseline": if args.no_baseline { Value::Null } else { json!(args.baseline) },
        "no_baseline": args.no_baseline,
        "thinking_steps": args.thinking_steps,
        "temperature": args.temperature,
        "first_seed": args.first_seed,
        "policy_seed": args.policy_seed,
        "deals": st.deals_completed,
        "net_win_rate": net_win_rate,
        "base_win_rate": base_win_rate,
        "net_utility_mean": net_u_mean,
        "base_utility_mean": base_u_mean,
        "duplicate_delta": delta,
        "advantage_vs_baseline": delta,
        "net_wins": st.net_wins,
        "net_losses": net_losses,
        "net_draws": st.net_draws,
        "net_answer_caps": st.net_answer_caps,
        "base_wins": if args.no_baseline { Value::Null } else { json!(st.base_wins) },
        "base_losses": base_losses,
        "base_draws": if args.no_baseline { Value::Null } else { json!(st.base_draws) },
        "base_answer_caps": if args.no_baseline { Value::Null } else { json!(st.base_answer_caps) },
        "net_answers": st.net_answers,
        "house_fallbacks": st.house_fallbacks,
        "fallback_rate": fb_rate,
        "refused": st.refused,
        "stalls": st.stalls,
        "latency_ms": {
            "mean": mean_ms,
            "p50": pct(0.50),
            "p95": pct(0.95),
            "p99": pct(0.99),
        },
        "elapsed_secs": started.elapsed().as_secs_f64(),
    });

    if let Some(parent) = args.out.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&args.out, serde_json::to_vec_pretty(&summary)?)?;
    eprintln!(
        "[eval4] seats={} teams={:?} K={} against={} deals={} | net_win={:.1}% (u={:.3}, caps={}) base_u={:?} delta={:?} | p50={:.1}ms p95={:.1}ms fb={} refused={} ({:.1}s)",
        args.seats,
        args.teams,
        args.thinking_steps,
        args.against,
        st.deals_completed,
        net_win_rate * 100.0,
        net_u_mean,
        st.net_answer_caps,
        base_u_mean,
        delta,
        pct(0.50),
        pct(0.95),
        st.house_fallbacks,
        st.refused,
        started.elapsed().as_secs_f64(),
    );
    Ok(())
}
