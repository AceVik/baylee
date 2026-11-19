//! Batched calls against single steps, and an FP16 export against its FP32
//! source, on the states of a real game. Both need an exported model:
//! `BAYLEE_TEST_MODEL=DIR`, and `BAYLEE_TEST_MODEL_FP16=DIR` for the second
//! (`BAYLEE_EP=cuda` runs them on the GPU). Run them with `--ignored`.
use super::inference::{StepRow, run_rows};
use super::*;
use crate::features4::EncodedState4;
use baylee_core::preset::AIProfile;

/// The model a test runs, when the environment names one. Its card catalog
/// may have drifted from this build's (`BAYLEE_TEST_CATALOG_DRIFT=1`), as
/// an older export's has.
fn model(var: &str) -> Option<NetPlayer4> {
    let dir = std::env::var(var).ok()?;
    Some(NetPlayer4::load_with(Path::new(&dir), catalog()).unwrap())
}

/// How a test model's card catalog binds to this build's.
fn catalog() -> CatalogPolicy4 {
    if std::env::var("BAYLEE_TEST_CATALOG_DRIFT").as_deref() == Ok("1") {
        CatalogPolicy4::Drift
    } else {
        CatalogPolicy4::Exact
    }
}

/// Every observation and decision row of the first `limit` steps of a 1v1
/// game that `net`, a model reading Encoder 7 states (7, 10 and later),
/// plays for both seats.
fn game_rows(net: &mut NetPlayer4, seed: u64, limit: usize) -> Vec<StepRow> {
    assert!(
        net.encoder_version == 7 || net.encoder_version >= 10,
        "the rows are encoded as Encoder 7"
    );
    let victory = crate::housedeck::HouseDeck::named("victory").unwrap();
    let allytifact = crate::housedeck::HouseDeck::named("allytifact").unwrap();
    let preset =
        crate::selfplay::table_for(seed, &[&victory, &allytifact], &[AIProfile::SHARP; 2], &[]);
    let mut session = baylee_gamehost::Session::new(&preset).unwrap();
    for seat in 0..2 {
        session.take_over(PlayerId::new(seat));
    }
    net.init_game(&preset);
    let teams = [None, None];
    let no_menu = EncodedCandidates4 {
        head: HeadKind4::SingleChoice,
        pending_kind: PendingKind4::Priority,
        min_choices: 0,
        max_choices: 0,
        candidates: Vec::new(),
        pairs: Vec::new(),
        pair_objects: Vec::new(),
    };
    let pack =
        |net: &NetPlayer4, state: &EncodedState4, cands: &EncodedCandidates4, seat: u8, k: u8| {
            StepRow::pack(
                state,
                cands,
                &net.h_states[&seat],
                k,
                net.encoder_version,
                net.rule_roots.as_deref(),
            )
            .unwrap()
        };
    let mut rows = Vec::new();
    while rows.len() < limit {
        let Some(player) = session.pending().asked() else {
            break;
        };
        let seat = player.get();
        let (pending, view) = session.view_for_with_context(player).unwrap();
        let observed = encode_state7(&view, None, net.knowledge.get(&seat), &teams);
        rows.push(pack(net, &observed, &no_menu, seat, 0));
        net.observe(&view).unwrap();
        let (state, cands) = net.policy_encoding(&view, &pending, &teams);
        rows.push(pack(net, &state, &cands, seat, net.thinking_steps));
        let action = net.act(&view, &pending, &teams);
        if session.act(player, action).is_err() {
            break;
        }
    }
    rows
}

/// Largest absolute difference over finite (unmasked) entries.
fn max_diff(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len());
    a.iter()
        .zip(b)
        .filter(|(x, y)| x.abs() < 1e4 && y.abs() < 1e4)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f32::max)
}

fn argmax(logits: &[f32]) -> usize {
    logits
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .map_or(0, |(i, _)| i)
}

fn single(net: &mut NetPlayer4, row: &StepRow) -> StepOutputs {
    let Backend::Owned(sessions) = &mut net.backend else {
        panic!("a test model owns its sessions");
    };
    let depth = depth_graph(row.k_val);
    let session = sessions.entry(depth).or_insert_with(|| {
        load_session(&net.model_dir.join(format!("model.k{depth}.onnx"))).unwrap()
    });
    run_rows(session, &[row], net.d_model)
        .unwrap()
        .pop()
        .unwrap()
}

#[test]
#[ignore = "needs BAYLEE_TEST_MODEL, an exported v4/v5 model directory"]
fn batched_calls_answer_every_row_as_its_single_step() {
    let Some(mut net) = model("BAYLEE_TEST_MODEL") else {
        return;
    };
    let rows = game_rows(&mut net, 4711, 96);
    assert!(rows.len() >= 32, "the game ended after {} rows", rows.len());
    let alone: Vec<StepOutputs> = rows.iter().map(|row| single(&mut net, row)).collect();
    let mut worst = 0.0_f32;
    for k in [0, net.thinking_steps] {
        let same: Vec<usize> = (0..rows.len()).filter(|&i| rows[i].k_val == k).collect();
        for call in same.chunks(16) {
            let batch: Vec<&StepRow> = call.iter().map(|&i| &rows[i]).collect();
            let Backend::Owned(sessions) = &mut net.backend else {
                unreachable!()
            };
            let session = sessions.get_mut(&depth_graph(k)).unwrap();
            let together = run_rows(session, &batch, net.d_model).unwrap();
            for (&i, out) in call.iter().zip(&together) {
                let reference = &alone[i];
                assert_eq!(out.cand_logits.len(), reference.cand_logits.len());
                assert_eq!(out.pair_logits.len(), reference.pair_logits.len());
                worst = worst
                    .max(max_diff(&out.h_out, &reference.h_out))
                    .max(max_diff(&out.cand_logits, &reference.cand_logits))
                    .max(max_diff(&out.pair_logits, &reference.pair_logits))
                    .max((out.scalar_pred - reference.scalar_pred).abs())
                    .max((out.value - reference.value).abs());
                if out.cand_logits.len() > 1 {
                    assert_eq!(
                        argmax(&out.cand_logits),
                        argmax(&reference.cand_logits),
                        "row {i}"
                    );
                }
            }
        }
    }
    eprintln!("{} rows, worst batched difference {worst:.3e}", rows.len());
    assert!(worst < 1e-3, "batched rows differ by {worst}");
}

/// Several servers answering calls of one row, sent from several threads at
/// once, give every row exactly its single step's outputs: what lets a
/// search's rollouts run on several cores and stay the unbatched search.
#[test]
#[ignore = "needs BAYLEE_TEST_MODEL, an exported v4/v5 model directory"]
fn served_calls_of_one_row_are_the_single_step_bit_for_bit() {
    let Some(mut net) = model("BAYLEE_TEST_MODEL") else {
        return;
    };
    let rows = game_rows(&mut net, 4711, 48);
    assert!(rows.len() >= 32, "the game ended after {} rows", rows.len());
    let alone: Vec<StepOutputs> = rows.iter().map(|row| single(&mut net, row)).collect();
    let served =
        BatchedNet4::load_served(&net.model_dir, BatchLimits4::for_rows(1), catalog(), 3).unwrap();
    let outputs: Vec<StepOutputs> = std::thread::scope(|scope| {
        let senders: Vec<_> = rows
            .chunks(rows.len().div_ceil(6))
            .map(|chunk| {
                let served = &served;
                scope.spawn(move || {
                    chunk
                        .iter()
                        .map(|row| served.run(row.clone()).unwrap())
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        senders
            .into_iter()
            .flat_map(|sender| sender.join().unwrap())
            .collect()
    });
    assert_eq!(outputs.len(), rows.len());
    for (i, (out, reference)) in outputs.iter().zip(&alone).enumerate() {
        assert_eq!(out.h_out, reference.h_out, "row {i}");
        assert_eq!(out.cand_logits, reference.cand_logits, "row {i}");
        assert_eq!(out.pair_logits, reference.pair_logits, "row {i}");
        assert_eq!(out.value.to_bits(), reference.value.to_bits(), "row {i}");
        assert_eq!(
            out.scalar_pred.to_bits(),
            reference.scalar_pred.to_bits(),
            "row {i}"
        );
    }
    let (calls, answered) = served.stats();
    assert_eq!((calls, answered), (rows.len() as u64, rows.len() as u64));
    assert!(served.call_seconds() > 0.0);
}

/// What a batched call costs on the CPU, per row, by rows per call, intra-op
/// threads and spinning: the measurement behind `eval4 --search-lockstep`
/// and `--inference-threads`. Prints a table; checks nothing but that every
/// call answers. `BAYLEE_BENCH_THREADS` (default `1,4`) names the thread
/// counts, `BAYLEE_BENCH_ROWS` (default 96) the rows each setting runs.
#[test]
#[ignore = "needs BAYLEE_TEST_MODEL; a timing, run in release"]
#[allow(
    clippy::cast_precision_loss,
    reason = "a timing table of a few hundred rows"
)]
fn batched_call_cost_on_the_cpu() {
    let Some(mut net) = model("BAYLEE_TEST_MODEL") else {
        return;
    };
    let mut rows = Vec::new();
    for seed in [4711, 4712, 4713] {
        let game = game_rows(&mut net, seed, 600);
        // The middle of a game, where rollouts play, not its first turns.
        rows.extend(game.into_iter().skip(200));
    }
    let threads: Vec<usize> = std::env::var("BAYLEE_BENCH_THREADS")
        .unwrap_or_else(|_| "1,4".into())
        .split(',')
        .map(|t| t.trim().parse().unwrap())
        .collect();
    let per_setting: usize = std::env::var("BAYLEE_BENCH_ROWS")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(96);
    for k in [0, net.thinking_steps] {
        let same: Vec<&StepRow> = rows.iter().filter(|r| r.k_val == k).collect();
        let n = per_setting.min(same.len());
        let tokens = same[..n].iter().map(|r| r.tokens()).sum::<usize>() as f64 / n as f64;
        eprintln!("k{k}: {n} rows, {tokens:.1} tokens a row on average");
        let path = net
            .model_dir
            .join(format!("model.k{}.onnx", depth_graph(k)));
        for &t in &threads {
            for spinning in if t > 1 { vec![true, false] } else { vec![true] } {
                let intra = IntraOp4 {
                    threads: t,
                    spinning,
                };
                let mut session = load_session_threads(&path, intra).unwrap();
                for row in same.iter().take(4) {
                    run_rows(&mut session, &[row], net.d_model).unwrap();
                }
                for b in [1, 2, 4, 8, 12, 16, 24, 32] {
                    let start = std::time::Instant::now();
                    for call in same[..n].chunks(b) {
                        let answered = run_rows(&mut session, call, net.d_model).unwrap();
                        assert_eq!(answered.len(), call.len());
                    }
                    let ms = start.elapsed().as_secs_f64() * 1e3;
                    eprintln!(
                        "k{k} threads {t} spinning {spinning} rows/call {b:>2}: \
                         {:7.2} ms a row, {:8.2} ms a call",
                        ms / n as f64,
                        ms / n.div_ceil(b) as f64
                    );
                }
            }
        }
    }
}

#[test]
#[ignore = "needs BAYLEE_TEST_MODEL and BAYLEE_TEST_MODEL_FP16 exports of one checkpoint"]
fn float16_export_chooses_as_its_float32_source() {
    let (Some(mut full), Some(mut half)) =
        (model("BAYLEE_TEST_MODEL"), model("BAYLEE_TEST_MODEL_FP16"))
    else {
        return;
    };
    assert_eq!(
        full.model_identity.source_checkpoint_sha256,
        half.model_identity.source_checkpoint_sha256
    );
    let mut rows = Vec::new();
    for seed in [4711, 4712, 4713, 4714] {
        rows.extend(game_rows(&mut full, seed, 256));
    }
    let (mut menus, mut agree) = (0_u32, 0_u32);
    let (mut worst_logit, mut worst_value, mut worst_memory) = (0.0_f32, 0.0_f32, 0.0_f32);
    for row in &rows {
        let a = single(&mut full, row);
        let b = single(&mut half, row);
        worst_memory = worst_memory.max(max_diff(&a.h_out, &b.h_out));
        worst_value = worst_value.max((a.value - b.value).abs());
        if row.k_val > 0 && a.cand_logits.len() > 1 {
            menus += 1;
            agree += u32::from(argmax(&a.cand_logits) == argmax(&b.cand_logits));
            worst_logit = worst_logit.max(max_diff(&a.cand_logits, &b.cand_logits));
        }
    }
    let agreement = f64::from(agree) / f64::from(menus.max(1));
    eprintln!(
        "{} rows, {menus} menus: top choice agrees {:.2}%, worst logit {worst_logit:.3e}, \
         value {worst_value:.3e}, memory {worst_memory:.3e}",
        rows.len(),
        agreement * 100.0
    );
    assert!(
        agreement >= 0.98,
        "FP16 changes {:.2}% of top choices",
        (1.0 - agreement) * 100.0
    );
    assert!(worst_value < 0.02, "FP16 moves a value by {worst_value}");
}
