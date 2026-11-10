//! The tactician inside the policy's dispatch: an imitating collection with no
//! exploration plays exactly what the tactical layer with its rule groups plays.
#![allow(clippy::cast_precision_loss, reason = "small test indices")]
use std::collections::BTreeSet;

use super::*;
use crate::netplay4::tactician::tests::on_a_table::{D, ME, drive, prior, table};
use crate::netplay4::tactician::tests_s1::windows;
use crate::netplay4::tactician::{Drawn4, Tactician4, TacticianMode4, rules_answer_fired};
use crate::netplay4::{ChoiceGuard4, NetPlayer4, StepEmbeddings4, TacticRules4, TacticalParts};

/// What the seat plays at this question: through the tactical layer with
/// `rules`, and the tactician when one is given.
fn play(
    tactician: Option<&mut Tactician4>,
    cands: &EncodedCandidates4,
    view: &PlayerView,
    pending: &Pending,
    logits: &[f32],
    embeddings: &StepEmbeddings4,
) -> (PlayerAction, TeacherTarget4) {
    let mut rng = Rng::new(31);
    let mut guard = ChoiceGuard4::default();
    guard.tactical = TacticalParts {
        priority: true,
        single: true,
        subset: true,
        pair: true,
        scalar: true,
    };
    let mut rules = TacticRules4::parse(baylee_train_ops::measured::RULE_GROUPS).unwrap();
    let (action, _, _, target) = PolicySampler4 {
        tactician,
        mode: PolicyMode4::Raw,
        temperature: 0.0,
        rng: &mut rng,
        active_source: None,
        guard: &mut guard,
        rules: &mut rules,
    }
    .select(
        cands,
        view,
        pending,
        PolicyOutputs4 {
            embeddings: Some(embeddings),
            cand_logits: logits,
            pair_logits: logits,
            scalar_pred: 0.5,
            value: 0.4,
        },
    )
    .unwrap();
    (action, target)
}

#[test]
fn imitation_without_exploration_plays_exactly_the_layer_with_its_groups() {
    let player = NetPlayer4::without_graphs(10, D);
    for (window, want) in windows() {
        let mut met = 0;
        'tables: for seed in 1..40_u64 {
            let mut session = table(seed);
            while let Some((pending, view)) = drive(&mut session, want) {
                let (_, cands) = player.policy_encoding(&view, &pending, &[None, None]);
                let units = window.units(&cands);
                let logits: Vec<f32> = (0..cands.candidates.len().max(cands.pairs.len()))
                    .map(|i| ((i as f32 + seed as f32) * 0.9).cos() * 2.0)
                    .collect();
                let embeddings = StepEmbeddings4 {
                    state: vec![0.5; D],
                    candidates: vec![0.25; cands.candidates.len() * D],
                    pairs: vec![0.125; cands.pairs.len() * D],
                };
                let layer = play(None, &cands, &view, &pending, &logits, &embeddings);
                let mut tactician = Tactician4::new(
                    BTreeSet::from([window]),
                    TacticianMode4::Imitate { epsilon: 0.0 },
                    prior(),
                )
                .unwrap();
                tactician.reseed(seed);
                let imitated = play(
                    Some(&mut tactician),
                    &cands,
                    &view,
                    &pending,
                    &logits,
                    &embeddings,
                );
                assert_eq!(imitated, layer, "{window:?}");
                let record = tactician.take_records().pop().expect("a window record");
                assert_eq!(record.drawn, Drawn4::Rules);
                assert_eq!(record.played, record.rules);
                assert_eq!(record.rules.len(), units);
                // DAgger reads the same answer on copies, where the layer
                // does not play.
                let rng = Rng::new(31);
                let step = crate::netplay4::tactician::TacticianStep {
                    cands: &cands,
                    view: &view,
                    pending: &pending,
                    cand_logits: &logits,
                    pair_logits: &logits,
                    value: 0.4,
                    embeddings: Some(&embeddings),
                    active_source: None,
                    rng: &rng,
                };
                let bundle = TacticRules4::parse(baylee_train_ops::measured::RULE_GROUPS).unwrap();
                let (read, fired) = rules_answer_fired(window, &step, &logits[..units], bundle);
                assert_eq!(read, record.rules, "{window:?}");
                // The groups the record attributes its label to are those
                // that changed something as the layer played.
                let fired: Vec<String> = fired.iter().map(|&g| g.to_owned()).collect();
                assert_eq!(record.fired, fired, "{window:?}");
                session.act_slice(ME, layer.0, 0).unwrap();
                met += 1;
                if met >= 6 {
                    break 'tables;
                }
            }
        }
        assert!(met > 0, "no {window:?} question met");
    }
}
