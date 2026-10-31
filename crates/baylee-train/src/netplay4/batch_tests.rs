//! Batched calls against single steps, and an FP16 export against its FP32
//! source, on the states of a real game. Both need an exported model:
//! `BAYLEE_TEST_MODEL=DIR`, and `BAYLEE_TEST_MODEL_FP16=DIR` for the second
//! (`BAYLEE_EP=cuda` runs them on the GPU). Run them with `--ignored`.
use super::inference::{StepRow, run_rows};
use super::*;
use crate::features4::EncodedState4;
use baylee_core::preset::AIProfile;

/// The model a test runs, when the environment names one.
fn model(var: &str) -> Option<NetPlayer4> {
    let dir = std::env::var(var).ok()?;
    Some(NetPlayer4::load(Path::new(&dir)).unwrap())
}

/// Every observation and decision row of the first `limit` steps of a 1v1
/// game that `net`, an Encoder 7 model, plays for both seats.
fn game_rows(net: &mut NetPlayer4, seed: u64, limit: usize) -> Vec<StepRow> {
    assert_eq!(net.encoder_version, 7, "the rows are encoded as Encoder 7");
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
                net.rule_roots.as_ref(),
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
                assert_eq!(out.1.len(), reference.1.len());
                assert_eq!(out.2.len(), reference.2.len());
                worst = worst
                    .max(max_diff(&out.0, &reference.0))
                    .max(max_diff(&out.1, &reference.1))
                    .max(max_diff(&out.2, &reference.2))
                    .max((out.3 - reference.3).abs())
                    .max((out.4 - reference.4).abs());
                if out.1.len() > 1 {
                    assert_eq!(argmax(&out.1), argmax(&reference.1), "row {i}");
                }
            }
        }
    }
    eprintln!("{} rows, worst batched difference {worst:.3e}", rows.len());
    assert!(worst < 1e-3, "batched rows differ by {worst}");
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
        worst_memory = worst_memory.max(max_diff(&a.0, &b.0));
        worst_value = worst_value.max((a.4 - b.4).abs());
        if row.k_val > 0 && a.1.len() > 1 {
            menus += 1;
            agree += u32::from(argmax(&a.1) == argmax(&b.1));
            worst_logit = worst_logit.max(max_diff(&a.1, &b.1));
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
