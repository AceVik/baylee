//! Stage S1: imitation of the tactical layer, the record it keeps and its
//! guard, and the answer a `tactician_head_v2` net builds.
#![allow(clippy::cast_precision_loss, reason = "small test indices")]
use std::path::PathBuf;

use serde_json::json;

use super::imitate::{IMITATION_CONTRACT, refuse_imitation};
use super::net::{NET_CONTRACT, TacticianNet4};
use super::tests::on_a_table::{D, ME, drive, prior, table};
use super::*;
use crate::netplay4::NetPlayer4;

/// The tiny `tactician_head_v2` net `tools/trainer/tactician_v3` exports for
/// these tests (d = 4, two passes).
fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/netplay4/tactician/fixtures/v2-tiny")
}

#[test]
fn imitation_runs_are_refused_wherever_their_contract_is_named() {
    let plain = json!({"run": "eval4-rollout", "choice_guard": "tactical",
        "tactician": {"record_contract": "tactician_records_v1"}});
    refuse_imitation(&plain).unwrap();
    for marked in [
        json!({"labels": IMITATION_CONTRACT}),
        json!({"tactician": {"record_contract": IMITATION_CONTRACT}}),
        json!({"runs": [{"contracts": ["x", IMITATION_CONTRACT]}]}),
    ] {
        let error = refuse_imitation(&marked).unwrap_err().to_string();
        assert!(
            error.contains("train the tactician and nothing else"),
            "{error}"
        );
    }
}

#[test]
fn an_unsure_net_plays_the_trunk_s_answer() {
    let (built, net) = (vec![0, 1, 1], vec![1, 0, 0]);
    let choose = build::choose;
    assert_eq!(
        choose(&built, true, 0.9, 0.5, &net),
        (built.clone(), Drawn4::Built)
    );
    assert_eq!(
        choose(&built, true, 0.5, 0.5, &net),
        (built.clone(), Drawn4::Built)
    );
    assert_eq!(
        choose(&built, true, 0.49, 0.5, &net),
        (net.clone(), Drawn4::Net)
    );
    assert_eq!(
        choose(&built, true, f32::NAN, 0.0, &net),
        (net.clone(), Drawn4::Net)
    );
    // An answer that does not decode is never played, however sure.
    assert_eq!(choose(&built, false, 1.0, 0.0, &net), (net, Drawn4::Net));
}

/// A record as stage 1 kept it: no key of stage S1 appears in its line.
fn stage_one_record() -> TacticianRecord4 {
    TacticianRecord4 {
        seat: 1,
        seq: 9,
        turn: 3,
        window: Window4::Block,
        value: 0.25,
        keys: vec!["a".into(), "b".into()],
        logits: vec![0.5, -1.0],
        net: vec![1, 0],
        rules: vec![0, 1],
        played: vec![1, 0],
        probability: 0.75,
        neighbors: 1,
        offered: vec![vec![0, 0]],
        drawn: Drawn4::Net,
        micros: 12,
        scores: None,
        groups: Vec::new(),
        label: None,
        rows_read_rules: None,
        built_logits: None,
        built: None,
        confidence: None,
        net_micros: None,
        fired: Vec::new(),
        state: vec![1.0; D],
        units: vec![2.0; 2 * D],
        rows: vec![3.0; 3 * ROW_TAIL],
    }
}

#[test]
fn stage_one_lines_keep_their_keys_and_s1_lines_round_trip() {
    let line = serde_json::to_value(stage_one_record()).unwrap();
    let keys: BTreeSet<&str> = line
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    let want: BTreeSet<&str> = [
        "seat",
        "seq",
        "turn",
        "window",
        "value",
        "keys",
        "logits",
        "net",
        "rules",
        "played",
        "probability",
        "neighbors",
        "offered",
        "drawn",
        "micros",
    ]
    .into_iter()
    .collect();
    assert_eq!(keys, want);
    let mut record = stage_one_record();
    record.groups = vec![0, 0];
    record.label = Some("rules".into());
    record.rows_read_rules = Some(false);
    record.built_logits = Some(vec![0.1, 0.2]);
    record.built = Some(vec![0, 1]);
    record.confidence = Some(0.8);
    record.net_micros = Some(5);
    record.drawn = Drawn4::Built;
    let text = serde_json::to_string(&record).unwrap();
    let back: TacticianRecord4 = serde_json::from_str(&text).unwrap();
    assert_eq!(serde_json::to_string(&back).unwrap(), text);
    assert_eq!(back.groups, record.groups);
    assert_eq!(back.confidence, Some(0.8));
    assert_eq!(back.drawn, Drawn4::Built);
    // The arrays travel in the blob, not the line.
    assert!(back.state.is_empty() && back.units.is_empty() && back.rows.is_empty());
}

#[test]
fn a_stage_one_head_and_a_v2_net_each_refuse_the_other_s_contract() {
    let dir = fixture();
    assert_eq!(head::contract_of(&dir).unwrap(), NET_CONTRACT);
    assert!(head::TacticianHead4::load(&dir).is_err());
    let net = TacticianNet4::load(&dir, 2).unwrap();
    assert_eq!(net.row_width(), 2 * D + ROW_TAIL);
    assert!(net.passes() >= 1);
    assert!((0.0..=1.0).contains(&net.threshold()));
    let n = 3;
    let tails: Vec<f32> = (0..(n + 1) * ROW_TAIL)
        .map(|i| (i as f32 * 0.37).sin())
        .collect();
    let (logits, confidence) = net.run(&[0.5; D], &vec![0.25; n * D], &tails, n).unwrap();
    assert_eq!(logits.len(), n);
    assert!((0.0..=1.0).contains(&confidence));
    // Same rows, same answer, from any session of the pool.
    assert_eq!(
        net.run(&[0.5; D], &vec![0.25; n * D], &tails, n).unwrap(),
        (logits, confidence)
    );
    assert!(
        net.run(&[0.5; D], &vec![0.25; n * D], &tails[ROW_TAIL..], n)
            .is_err()
    );
}

/// The live questions of `window` on some tables, each with a step to
/// answer: `check` runs on each, and returns the answer to play on.
fn on_tables(
    window: Window4,
    want: impl Fn(&PlayerView, &Pending) -> bool + Copy,
    mut check: impl FnMut(&TacticianStep<'_>, u64) -> TeacherTarget4,
) {
    let player = NetPlayer4::without_graphs(10, D);
    let mut met = 0;
    for seed in 1..40_u64 {
        let mut session = table(seed);
        while let Some((pending, view)) = drive(&mut session, want) {
            let (_, cands) = player.policy_encoding(&view, &pending, &[None, None]);
            let units = window.units(&cands);
            let logits: Vec<f32> = (0..units)
                .map(|i| ((i as f32 + seed as f32) * 0.7).sin())
                .collect();
            let embeddings = StepEmbeddings4 {
                state: vec![0.5; D],
                candidates: vec![0.25; cands.candidates.len() * D],
                pairs: vec![0.125; cands.pairs.len() * D],
            };
            let rng = Rng::new(seed);
            let step = TacticianStep {
                cands: &cands,
                view: &view,
                pending: &pending,
                cand_logits: &logits,
                pair_logits: &logits,
                value: 0.5,
                embeddings: Some(&embeddings),
                active_source: None,
                rng: &rng,
            };
            let target = check(&step, seed);
            let action = decode_action4(&cands, &view, &pending, &target).unwrap();
            assert_eq!(pending.answer_fault(&action), None);
            session.act_slice(ME, action, 0).unwrap();
            met += 1;
            if met >= 6 {
                return;
            }
        }
    }
    assert!(met > 0, "no {window:?} question met");
}

fn attacks(_: &PlayerView, p: &Pending) -> bool {
    matches!(p, Pending::ChooseAttackers { attackers, .. } if attackers.len() >= 2)
}

fn blocks(_: &PlayerView, p: &Pending) -> bool {
    matches!(p, Pending::ChooseBlockers { blockers, .. } if !blockers.is_empty())
}

fn targets(_: &PlayerView, p: &Pending) -> bool {
    matches!(p, Pending::ChooseTargets { options, player_options, max: 1, .. }
        if options.len() + player_options.len() >= 2)
}

/// Whether a question opens the window it is paired with.
pub(in crate::netplay4) type Opens = fn(&PlayerView, &Pending) -> bool;

/// The windows with the questions that open them.
pub(in crate::netplay4) fn windows() -> [(Window4, Opens); 3] {
    [
        (Window4::Attack, attacks),
        (Window4::Block, blocks),
        (Window4::Target, targets),
    ]
}

/// The tactician in `mode` at `window`, seeded.
fn tactician(window: Window4, mode: TacticianMode4, seed: u64) -> Tactician4 {
    let mut tactician = Tactician4::new(BTreeSet::from([window]), mode, prior()).unwrap();
    tactician.reseed(seed);
    tactician
}

/// The rows carry nothing of the layer's answer.
fn assert_rows_without_rules(rows: &[f32], units: usize) {
    assert_eq!(rows.len(), (units + 1) * ROW_TAIL);
    for row in rows.chunks(ROW_TAIL) {
        assert_eq!((row[5], row[11]), (0.0, 0.0));
    }
}

#[test]
fn without_exploration_imitation_plays_the_layer_s_own_answer() {
    for (window, want) in windows() {
        on_tables(window, want, |step, seed| {
            let mut tactician = tactician(window, TacticianMode4::Imitate { epsilon: 0.0 }, seed);
            let units = window.units(step.cands);
            let logits = &(if window == Window4::Block {
                step.pair_logits
            } else {
                step.cand_logits
            })[..units];
            let rules = rules_answer(window, step, logits);
            let layer = target_of(window, &rules);
            // Before the layer, an imitating tactician answers nothing.
            assert!(tactician.answer(step).unwrap().is_none());
            let played = tactician.imitate(step, &layer, &[]).unwrap().unwrap();
            assert_eq!(played, layer);
            let record = tactician.take_records().pop().unwrap();
            assert_eq!(record.rules, rules);
            assert_eq!(record.played, rules);
            assert_eq!(record.drawn, Drawn4::Rules);
            assert!((record.probability - 1.0).abs() < 1e-12);
            assert_eq!(record.label.as_deref(), Some("rules"));
            assert_eq!(record.rows_read_rules, Some(false));
            assert_eq!(record.groups.len(), units);
            assert_eq!(record.units.len(), units * D);
            assert_rows_without_rules(&record.rows, units);
            if window == Window4::Target {
                assert!(record.groups.iter().all(|&g| g == 0));
            }
            played
        });
    }
}

#[test]
fn imitation_explores_into_legal_answers_with_exact_probabilities() {
    let epsilon = 0.6;
    for (window, want) in windows() {
        on_tables(window, want, |step, seed| {
            let units = window.units(step.cands);
            let logits = &(if window == Window4::Block {
                step.pair_logits
            } else {
                step.cand_logits
            })[..units];
            let rules = rules_answer(window, step, logits);
            let layer = target_of(window, &rules);
            let question = Tactician4::new(
                BTreeSet::from([window]),
                TacticianMode4::Imitate { epsilon },
                prior(),
            )
            .unwrap()
            .question(step)
            .unwrap()
            .unwrap();
            let mut seen: Vec<Vec<u8>> = Vec::new();
            let mut total = 0.0;
            let prior = prior();
            for k in 0..500 {
                let draw = (f64::from(k) + 0.5) / 500.0;
                let (target, record) =
                    imitate::answer(step, &question, &layer, epsilon, draw, &[], &prior).unwrap();
                assert_eq!(record.rules, rules);
                assert!(exact_action(window, step, &record.played).is_some());
                if record.played == rules {
                    assert_eq!(target, layer);
                } else {
                    assert_eq!(target, target_of(window, &record.played));
                }
                if !seen.contains(&record.played) {
                    total += record.probability;
                    seen.push(record.played);
                }
            }
            assert!((total - 1.0).abs() < 1e-9, "{window:?}: {total}");
            let mut tactician = tactician(window, TacticianMode4::Imitate { epsilon }, seed);
            tactician.imitate(step, &layer, &[]).unwrap().unwrap()
        });
    }
}

#[test]
fn any_logits_decode_to_a_legal_answer() {
    for (window, want) in windows() {
        on_tables(window, want, |step, seed| {
            let units = window.units(step.cands);
            let mut rng = Rng::new(seed ^ 0x51);
            for _ in 0..16 {
                let logits: Vec<f32> = (0..units)
                    .map(|_| (rng.unit() as f32 - 0.5) * 12.0)
                    .collect();
                let built = net_answer(window, step, &logits);
                assert!(exact_action(window, step, &built).is_some(), "{window:?}");
            }
            let logits: Vec<f32> = vec![-3.0; units];
            to_target(window, step, &net_answer(window, step, &logits)).unwrap()
        });
    }
}

#[test]
fn a_v2_net_builds_legal_answers_and_falls_back_to_the_trunk_when_unsure() {
    let net = Arc::new(TacticianNet4::load(&fixture(), 1).unwrap());
    for (window, want) in windows() {
        on_tables(window, want, |step, seed| {
            let units = window.units(step.cands);
            let logits = &(if window == Window4::Block {
                step.pair_logits
            } else {
                step.cand_logits
            })[..units];
            let trunk = net_answer(window, step, logits);
            let sure = TacticianMode4::Build {
                net: net.clone(),
                threshold: 0.0,
                label: None,
            };
            let mut builder = tactician(window, sure, seed);
            let target = builder.answer(step).unwrap().unwrap();
            let record = builder.take_records().pop().unwrap();
            assert_eq!(record.drawn, Drawn4::Built);
            assert_eq!(Some(&record.played), record.built.as_ref());
            assert!(exact_action(window, step, &record.played).is_some());
            assert_eq!(record.built_logits.as_ref().map(Vec::len), Some(units));
            assert_rows_without_rules(&record.rows, units);
            let confidence = record.confidence.unwrap();
            let unsure = TacticianMode4::Build {
                net: net.clone(),
                threshold: 1.0,
                label: None,
            };
            let mut doubter = tactician(window, unsure, seed);
            let fallback = doubter.answer(step).unwrap().unwrap();
            let record = doubter.take_records().pop().unwrap();
            if confidence < 1.0 {
                assert_eq!(record.drawn, Drawn4::Net);
                assert_eq!(record.played, trunk);
                assert_eq!(fallback, target_of(window, &trunk));
            }
            target
        });
    }
}

#[test]
fn dagger_plays_the_student_and_keeps_the_layer_s_answer_as_the_label() {
    let net = Arc::new(TacticianNet4::load(&fixture(), 1).unwrap());
    let bundle = TacticRules4::parse(baylee_train_ops::measured::RULE_GROUPS).unwrap();
    for (window, want) in windows() {
        on_tables(window, want, |step, seed| {
            let units = window.units(step.cands);
            let logits = &(if window == Window4::Block {
                step.pair_logits
            } else {
                step.cand_logits
            })[..units];
            let relabel = TacticianMode4::Build {
                net: net.clone(),
                threshold: 0.0,
                label: Some(bundle),
            };
            let mut student = tactician(window, relabel, seed);
            let target = student.answer(step).unwrap().unwrap();
            let record = student.take_records().pop().unwrap();
            assert_eq!(record.drawn, Drawn4::Built);
            assert_eq!(Some(&record.played), record.built.as_ref());
            assert_eq!(
                record.rules,
                rules_answer_with(window, step, logits, bundle)
            );
            assert_eq!(record.label.as_deref(), Some("rules"));
            assert_eq!(record.rows_read_rules, Some(false));
            assert_rows_without_rules(&record.rows, units);
            target
        });
    }
}
