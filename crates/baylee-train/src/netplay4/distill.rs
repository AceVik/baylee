//! Whole-net distillation labels (`bundle_distillation_v1`): at every
//! decision the net answers through its policy, the net's raw greedy answer
//! `N` and the tactical layer's answer `R` under a set of rule groups, read on
//! copies at the same view and logits exactly as `--choice-guard tactical
//! --tactics-rules GROUPS` plays it, with the groups that changed it.
//!
//! Whoever plays (the layer itself, or the net as a student, `DAgger`), the
//! labels are what the layer would answer at the states reached. `eval4
//! --distill-labels GROUPS` keeps one entry per step of the net's seats in
//! `games.jsonl` (`distill_sidecar`), aligned with `rollout_sidecar`; `convert4
//! --bundle-distill` makes `R` the policy target. Like the tactician's
//! imitation labels these are the layer's answers: every other reader of a run
//! refuses one that names the contract
//! (`baylee_train_ops::tactician_labels::refuse_imitation`).
//!
//! Covered: every head the policy answers, priority (a single choice at
//! `Pending::Priority`), other single choices (targets, modes, options),
//! subsets (attackers, several targets, cards), pairs (blockers), and
//! numbers. Forced answers, fallbacks and trigger orders carry no label.

use baylee_core::ids::ObjectId;
use baylee_engine::choice::Pending;
use baylee_view::PlayerView;

use super::policy::{layer_scalar, sample_categorical};
use super::{ChoiceGuard4, TacticRules4, TacticalParts, choices, tactics};
use crate::deckgen::Rng;
use crate::features4::{
    EncodedCandidates4, HeadKind4, PendingKind4, TeacherTarget4, decode_action4,
    denormalize_scalar_choice, normalize_scalar_choice, sample_pairs_constrained,
    sample_subset_constrained,
};

pub use crate::convert4::distill_labels::{DistillLabel4, DistillStep4, menu_of, menu_signature};
pub use baylee_train_ops::tactician_labels::DISTILL_CONTRACT;

/// A net's distillation recorder.
#[derive(Clone, Debug)]
pub struct Distill4 {
    rules: TacticRules4,
    last: Option<DistillLabel4>,
    steps: Vec<DistillStep4>,
}

impl Distill4 {
    /// Labels under the rule groups `rules`.
    #[must_use]
    pub const fn new(rules: TacticRules4) -> Self {
        Self {
            rules,
            last: None,
            steps: Vec::new(),
        }
    }

    /// Forgets the last decision's labels (a new decision begins).
    pub(super) fn begin(&mut self) {
        self.last = None;
    }

    /// The label groups.
    pub(super) const fn rules(&self) -> TacticRules4 {
        self.rules
    }

    /// Keeps the labels of the decision just answered.
    pub(super) fn keep(&mut self, label: DistillLabel4) {
        self.last = Some(label);
    }

    /// The last decision's labels, taken.
    pub fn take_label(&mut self) -> Option<DistillLabel4> {
        self.last.take()
    }

    /// Records a step of this net's seat: the last decision's labels when the
    /// policy answered it.
    pub fn step(&mut self, seat: u8, decision_seq: u64, record_byte_offset: usize, answered: bool) {
        let label = self.last.take().filter(|_| answered);
        self.steps.push(DistillStep4 {
            seat,
            decision_seq,
            record_byte_offset,
            label,
        });
    }

    /// The game's steps, taken.
    pub fn take_steps(&mut self) -> Vec<DistillStep4> {
        std::mem::take(&mut self.steps)
    }
}

/// What a decision offers, read once for its labels.
pub(super) struct Question<'a> {
    pub(super) cands: &'a EncodedCandidates4,
    pub(super) view: &'a PlayerView,
    pub(super) pending: &'a Pending,
    pub(super) cand_logits: &'a [f32],
    pub(super) pair_logits: &'a [f32],
    pub(super) scalar_pred: f32,
    pub(super) active_source: Option<(ObjectId, Option<u32>)>,
    pub(super) rng: &'a Rng,
}

/// The kind of a decision by its head and question.
fn kind_of(head: HeadKind4, pending: &Pending) -> &'static str {
    match head {
        HeadKind4::SingleChoice if matches!(pending, Pending::Priority { .. }) => "priority",
        HeadKind4::SingleChoice => "single",
        HeadKind4::SubsetPick => "subset",
        HeadKind4::PairPick => "pair",
        HeadKind4::ScalarValue => "scalar",
    }
}

/// A scalar target from a normalized prediction.
fn scalar_target(cands: &EncodedCandidates4, pred: f32) -> TeacherTarget4 {
    let chosen = denormalize_scalar_choice(pred, cands.min_choices, cands.max_choices);
    TeacherTarget4 {
        scalar_norm: normalize_scalar_choice(chosen, cands.min_choices, cands.max_choices),
        ..TeacherTarget4::default()
    }
}

/// A subset or pair target from a constrained sample.
fn set_target((mask, free_mask, seq, _): (Vec<u8>, Vec<u8>, Vec<u16>, f32)) -> TeacherTarget4 {
    TeacherTarget4 {
        multi_mask: mask,
        free_mask,
        pick_sequence: seq,
        ..TeacherTarget4::default()
    }
}

/// `N`: the raw greedy answer, as the raw policy plays it at temperature 0.
pub(super) fn net_target(q: &Question<'_>) -> TeacherTarget4 {
    let mut rng = q.rng.clone();
    let cands = q.cands;
    match cands.head {
        HeadKind4::SingleChoice => TeacherTarget4 {
            single_idx: sample_categorical(&q.cand_logits[..cands.candidates.len()], 0.0, &mut rng)
                .0 as u16,
            ..TeacherTarget4::default()
        },
        HeadKind4::SubsetPick => set_target(sample_subset_constrained(
            &q.cand_logits[..cands.candidates.len()],
            cands,
            q.pending,
            0.0,
            &mut rng,
        )),
        HeadKind4::PairPick => set_target(sample_pairs_constrained(
            &q.pair_logits[..cands.pairs.len()],
            cands,
            0.0,
            &mut rng,
        )),
        HeadKind4::ScalarValue => scalar_target(cands, q.scalar_pred.clamp(0.0, 1.0)),
    }
}

/// `R`: the tactical layer's answer under `rules`, as `--choice-guard
/// tactical --tactics-rules` plays it, read on copies; with the groups that
/// changed something on the way.
pub(super) fn rules_target(q: &Question<'_>, rules: TacticRules4) -> (TeacherTarget4, Vec<String>) {
    let mut rng = q.rng.clone();
    let before = rules;
    let mut rules = rules;
    let mut guard = ChoiceGuard4::default();
    guard.tactical = TacticalParts {
        priority: true,
        single: true,
        subset: true,
        pair: true,
        scalar: true,
    };
    let cands = q.cands;
    let target = match cands.head {
        HeadKind4::SingleChoice => {
            let n = cands.candidates.len();
            let (chosen, _) = tactics::rules::single_choice(
                &mut rules,
                cands,
                q.cand_logits,
                q.view,
                q.pending,
                q.active_source,
                0.0,
                &mut rng,
            );
            let chosen = tactics::guard::review(
                &mut guard,
                cands,
                &q.cand_logits[..n],
                q.view,
                q.pending,
                chosen,
            );
            TeacherTarget4 {
                single_idx: chosen as u16,
                ..TeacherTarget4::default()
            }
        }
        HeadKind4::SubsetPick => {
            let n = cands.candidates.len();
            let mut adjusted = choices::adjust_subset_logits(
                cands,
                &q.cand_logits[..n],
                q.view,
                q.pending,
                q.active_source,
            );
            tactics::rules::subset(
                &mut rules,
                cands,
                &mut adjusted,
                q.view,
                q.pending,
                q.active_source,
            );
            set_target(sample_subset_constrained(
                &adjusted, cands, q.pending, 0.0, &mut rng,
            ))
        }
        HeadKind4::PairPick => {
            let mut adjusted = choices::adjust_pair_logits(
                cands,
                &q.pair_logits[..cands.pairs.len()],
                q.view,
                q.pending,
            );
            tactics::rules::pairs(&mut rules, cands, &mut adjusted, q.view, q.pending);
            set_target(sample_pairs_constrained(&adjusted, cands, 0.0, &mut rng))
        }
        HeadKind4::ScalarValue => {
            scalar_target(cands, layer_scalar(q.view, q.pending, q.scalar_pred))
        }
    };
    let fired = rules
        .fired_since(&before)
        .into_iter()
        .map(str::to_owned)
        .collect();
    (target, fired)
}

/// Whether two targets of this head name different answers.
fn targets_differ(head: HeadKind4, a: &TeacherTarget4, b: &TeacherTarget4) -> bool {
    match head {
        HeadKind4::SingleChoice => a.single_idx != b.single_idx,
        HeadKind4::SubsetPick | HeadKind4::PairPick => a.multi_mask != b.multi_mask,
        HeadKind4::ScalarValue => (a.scalar_norm - b.scalar_norm).abs() > 1e-6,
    }
}

/// The labels of one decision whose policy answered `executed`.
pub(super) fn label(
    q: &Question<'_>,
    rules: TacticRules4,
    executed: &TeacherTarget4,
) -> DistillLabel4 {
    let net = net_target(q);
    let (rules_t, fired) = rules_target(q, rules);
    let decode = |target: &TeacherTarget4| decode_action4(q.cands, q.view, q.pending, target);
    let net_action = decode(&net);
    let rules_action = decode(&rules_t);
    let n_ne_r = match (&net_action, &rules_action) {
        (Some(n), Some(r)) => n != r,
        _ => targets_differ(q.cands.head, &net, &rules_t),
    };
    DistillLabel4 {
        kind: kind_of(q.cands.head, q.pending).to_owned(),
        pending_kind: PendingKind4::of(q.pending) as u8,
        menu_sha256: menu_of(q.cands),
        rules: rules_t,
        rules_action,
        net,
        net_action,
        executed: executed.clone(),
        fired,
        n_ne_r,
    }
}
