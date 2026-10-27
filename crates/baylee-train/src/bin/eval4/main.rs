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
use baylee_train::netplay4::{
    ModelIdentity, NetPlayer4, PolicyMode4, inspect_model_identity, valid_raw_temperature,
};
use baylee_train::selfplay::table_for;
use baylee_train::working::{Working, repo_root};
use clap::Parser;
use flate2::Compression;
use flate2::write::GzEncoder;
use serde_json::{Value, json};

include!("options.rs");
include!("runtime.rs");
include!("game.rs");
include!("decks.rs");

#[allow(
    clippy::too_many_lines,
    reason = "Evaluation setup, workers and aggregate report"
)]
#[allow(
    clippy::cast_precision_loss,
    reason = "Floating-point statistical rates and latency quantiles"
)]
fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    if args.raw_policy && (args.legacy_v3 || !valid_raw_temperature(args.temperature)) {
        bail!("--raw-policy requires v4 and temperature 0 or >0.35");
    }
    let policy_mode = if args.raw_policy {
        PolicyMode4::Raw
    } else {
        PolicyMode4::Tactical
    };
    let model_identity = if args.legacy_v3 {
        None
    } else {
        Some(inspect_model_identity(&args.model)?)
    };
    if args.legacy_v3
        && (args.record_out.is_some() || args.temperature != 0.0 || args.thinking_steps != 1)
    {
        bail!("--legacy-v3 supports greedy K=1 evaluation only, without rollout recording");
    }
    if !(2..=8).contains(&args.seats) {
        bail!("--seats must be between 2 and 8");
    }
    let teams: Vec<Option<u8>> = if args.teams.is_empty() {
        vec![None; args.seats]
    } else {
        if args.teams.len() != args.seats {
            bail!("--teams length must equal --seats");
        }
        args.teams.iter().map(|&t| (t != 0).then_some(t)).collect()
    };

    let opp_profile = parse_profile(&args.against)?;
    let base_profile = parse_profile(&args.baseline)?;

    let working = Working::scan(&repo_root())?;
    let shape = match args.shape.as_str() {
        "constructed" | "limited" => Shape::Constructed,
        "highlander" => Shape::Highlander,
        "commander" => Shape::Commander,
        other => bail!("unknown shape {other}"),
    };

    let held = deckgen::held_out(&working, 20);
    let empty_held = BTreeSet::new();
    let pool = if args.held_out_cards {
        // Include all working cards (including the held-out 5%) and seed from the held-out region
        deckgen::pool_for_working(&working, &empty_held)
    } else {
        deckgen::pool_for_working(&working, &held)
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
    let seed_base = if args.held_out_cards {
        99_999_u64
    } else {
        7_777_u64
    };
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
        bail!(
            "could not load/generate enough decks ({} < {})",
            decks.len(),
            args.seats
        );
    }

    if let Some(rec_dir) = &args.record_out {
        anyhow::ensure!(
            !rec_dir.exists() || fs::read_dir(rec_dir)?.next().is_none(),
            "--record-out must be new or empty; existing records cannot acquire new provenance"
        );
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
                "policy_mode": policy_mode,
                "policy_contract": policy_mode.contract(),
                "policy_provenance_version": 1,
                "model_family": model_identity.as_ref().map(|identity|identity.model_family),
                "model_identity": model_identity,
                "evaluator_build": baylee_build::short(),
                "evaluator_revision": baylee_build::COMMIT,
                "evaluator_dirty": baylee_build::DIRTY,
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
    let end_deal = args
        .deal_offset
        .checked_add(args.deals)
        .context("deal offset plus count overflows")?;
    let next_deal = Arc::new(AtomicU64::new(args.deal_offset));
    let stats_arc = Arc::new(Mutex::new(EvalStats::default()));
    let games_lines = Arc::new(Mutex::new(
        args.record_out
            .as_ref()
            .map(|dir| File::create(dir.join("games.jsonl")).map(BufWriter::new))
            .transpose()?,
    ));

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
        let deals = end_deal;
        let seats = args.seats;
        let first_seed = args.first_seed;
        let policy_seed = args.policy_seed;
        let k_steps = args.thinking_steps;
        let temp = args.temperature;
        let fair = args.fair_scouting;
        let no_base = args.no_baseline;
        let legacy = args.legacy_v3;
        let expected_identity = model_identity.clone();

        handles.push(std::thread::spawn(move || -> anyhow::Result<()> {
            let mut net = load_player(
                &model_path,
                legacy,
                k_steps,
                temp,
                policy_mode,
                expected_identity.as_ref(),
            )?;

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
                for (s, profile) in profiles.iter_mut().enumerate() {
                    if is_net_seat(s, focal_seat, &teams_arc) {
                        *profile = base_profile;
                    }
                }

                let deal_game_id = format!("eval4-deal-{table_idx:05}");
                let rec_game_id = format!("eval4-{d:05}");
                if let Some(dir) = &rec_out {
                    // Written before entering the engine, so aborts/hangs have
                    // a reproducible deal even without a completed record.
                    fs::write(
                        dir.join(format!("worker-{worker_id:02}.json")),
                        serde_json::to_vec_pretty(&json!({"status": "running", "deal": d,
                            "seed": seed, "focal_seat": focal_seat,
                            "decks": table_decks.iter().map(|deck| &deck.key).collect::<Vec<_>>(),
                            "policy_seed": policy_seed, "model": model_path}))?,
                    )?;
                }
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
                    Some(net.as_mut()),
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
                    if let Some(index) = games_lines.lock().expect("games lock").as_mut() {
                        writeln!(index, "{line}")?;
                        index.flush()?;
                    }
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
                st.deal_details.push(json!({"deal": d, "table": table_idx,
                    "seed": seed, "focal_seat": focal_seat,
                    "net_utility": net_u, "base_utility": if no_base { None } else { Some(base_u) },
                    "net_capped": net_capped, "base_capped": base_capped,
                    "refused": ref_cnt, "stalls": st_cnt, "house_fallbacks": fb,
                    "decks": table_decks.iter().map(|deck| &deck.key).collect::<Vec<_>>()}));
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
            let coverage = net.coverage();
            for pk in 0..coverage.counts.len() {
                for sk in 0..4 {
                    st.coverage.counts[pk][sk] += coverage.counts[pk][sk];
                }
            }
            Ok(())
        }));
    }

    for h in handles {
        h.join().expect("worker panicked")?;
    }

    let mut st = stats_arc.lock().expect("stats lock");
    st.deal_details.sort_by_key(|deal| deal["deal"].as_u64());
    st.latencies_ms.sort_by(f64::total_cmp);
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

    let mut summary = json!({
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
        "deal_offset": args.deal_offset,
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
    summary["policy_mode"] = if args.legacy_v3 {
        json!("legacy_v3")
    } else {
        json!(policy_mode)
    };
    summary["policy_contract"] = if args.legacy_v3 {
        Value::Null
    } else {
        json!(policy_mode.contract())
    };
    summary["policy_provenance_version"] = if args.legacy_v3 {
        Value::Null
    } else {
        json!(1)
    };
    summary["house_counts_include_tactical_demonstrations"] =
        json!(!args.legacy_v3 && policy_mode == PolicyMode4::Tactical);
    summary["deal_details"] = json!(st.deal_details);
    summary["coverage_matrix"] = json!(st.coverage.counts);
    summary["model_family"] = json!(
        model_identity
            .as_ref()
            .map_or("v3-expert", |identity| identity.model_family)
    );
    summary["model_identity"] = json!(model_identity);
    summary["evaluator_build"] = json!(baylee_build::short());
    summary["evaluator_revision"] = json!(baylee_build::COMMIT);
    summary["evaluator_dirty"] = json!(baylee_build::DIRTY);
    summary["latency_scope"] = json!("decision inference only; observation updates are excluded");

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
