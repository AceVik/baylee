//! Whole-net distillation labels inside the policy's dispatch: `R` read on
//! copies is what the layer with its groups plays, at every kind of decision,
//! and a student playing raw plays `N`, never `R`.
#![allow(clippy::cast_precision_loss, reason = "small test indices")]
use std::collections::BTreeSet;

use super::*;
use crate::netplay4::distill::{Distill4, DistillLabel4};
use crate::netplay4::tactician::tests::on_a_table::{D, ME, drive, table};
use crate::netplay4::{ChoiceGuard4, NetPlayer4, TacticRules4, TacticalParts};

/// The answer at this question, by the layer with the bundle (`layer`) or by
/// the raw net, with the distillation label of it.
fn answer(
    layer: bool,
    cands: &EncodedCandidates4,
    view: &PlayerView,
    pending: &Pending,
    logits: &[f32],
) -> (PlayerAction, TeacherTarget4, DistillLabel4) {
    let mut rng = Rng::new(17);
    let mut guard = ChoiceGuard4::default();
    let bundle = TacticRules4::parse(baylee_train_ops::measured::RULE_GROUPS).unwrap();
    if layer {
        guard.tactical = TacticalParts {
            priority: true,
            single: true,
            subset: true,
            pair: true,
            scalar: true,
        };
    }
    let mut rules = if layer {
        bundle
    } else {
        TacticRules4::default()
    };
    let mut distill = Distill4::new(bundle);
    let (action, _, _, target) = PolicySampler4 {
        tactician: None,
        distill: Some(&mut distill),
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
            embeddings: None,
            cand_logits: logits,
            pair_logits: logits,
            scalar_pred: 0.37,
            value: 0.4,
        },
    )
    .unwrap();
    let label = distill
        .take_label()
        .expect("every policy answer is labelled");
    (action, target, label)
}

fn same(head: HeadKind4, a: &TeacherTarget4, b: &TeacherTarget4) -> bool {
    match head {
        HeadKind4::SingleChoice => a.single_idx == b.single_idx,
        HeadKind4::SubsetPick | HeadKind4::PairPick => a.multi_mask == b.multi_mask,
        HeadKind4::ScalarValue => (a.scalar_norm - b.scalar_norm).abs() < 1e-6,
    }
}

#[test]
fn r_read_on_copies_is_what_the_layer_plays_and_a_student_plays_n() {
    let player = NetPlayer4::without_graphs(10, D);
    let mut kinds = BTreeSet::new();
    let mut differing = 0;
    let mut decisions = 0;
    for seed in 1..12_u64 {
        let mut here = 0;
        let mut session = table(seed);
        while let Some((pending, view)) = drive(&mut session, |_, _| true) {
            let (_, cands) = player.policy_encoding(&view, &pending, &[None, None]);
            let forced = match cands.head {
                HeadKind4::SingleChoice => cands.candidates.len() <= 1,
                HeadKind4::SubsetPick => cands.candidates.is_empty(),
                HeadKind4::PairPick => cands.pairs.is_empty(),
                HeadKind4::ScalarValue => cands.max_choices <= cands.min_choices,
            };
            if forced || matches!(pending, Pending::Arrange { .. }) {
                let action = session.house_action(ME).unwrap();
                session.act_slice(ME, action, 0).unwrap();
                continue;
            }
            let logits: Vec<f32> = (0..cands.candidates.len().max(cands.pairs.len()))
                .map(|i| ((i as f32 + seed as f32 + decisions as f32) * 0.77).sin() * 3.0)
                .collect();
            let (layer_action, layer_target, layer_label) =
                answer(true, &cands, &view, &pending, &logits);
            // The layer playing: R is its own answer, read on copies.
            assert!(
                same(cands.head, &layer_label.rules, &layer_target),
                "{pending:?}"
            );
            assert!(same(cands.head, &layer_label.executed, &layer_target));
            // A student playing raw: it plays N, and R is still the layer's.
            let (raw_action, raw_target, raw_label) =
                answer(false, &cands, &view, &pending, &logits);
            assert!(same(cands.head, &raw_label.net, &raw_target), "{pending:?}");
            assert!(
                same(cands.head, &raw_label.rules, &layer_target),
                "{pending:?}"
            );
            assert_eq!(raw_label.fired, layer_label.fired);
            assert_eq!(raw_label.n_ne_r, layer_label.n_ne_r);
            if raw_label.n_ne_r {
                differing += 1;
                assert_ne!(Some(&raw_action), raw_label.rules_action.as_ref());
            } else {
                assert_eq!(Some(&raw_action), raw_label.rules_action.as_ref());
            }
            assert_eq!(
                raw_label.menu_sha256,
                crate::netplay4::distill::menu_of(&cands)
            );
            kinds.insert(raw_label.kind.clone());
            decisions += 1;
            here += 1;
            session.act_slice(ME, layer_action, 0).unwrap();
            if here >= 250 {
                break;
            }
        }
    }
    assert!(differing > 0, "no decision where N and R differ");
    for kind in ["priority", "single", "subset", "pair"] {
        assert!(kinds.contains(kind), "no {kind} decision met: {kinds:?}");
    }
}
