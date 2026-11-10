//! Rule proposals as recorded exploration in on-policy collection (`eval4
//! --explore-rules GROUPS --explore-epsilon EPS`; the contract is
//! [`baylee_train_ops::exploration`], `docs/trained-ai.md` §"Rule proposals
//! as exploration").
//!
//! At every decision the raw sampler answers for the learner, the explorer
//! reads what the tactical layer with its groups answers on the same logits,
//! as `--choice-guard tactical --tactics-rules` plays it at temperature 0,
//! on copies: the net's sampling stream, the rule groups and the guard are
//! cloned, so nothing the net carries moves and the net's own draw is the one
//! it makes without exploration. The explorer then draws one coin from its
//! own stream, seeded per game from the learner's policy seed. Where the
//! layer's answer differs from the net's sample and the coin falls under ε,
//! the layer's answer is played instead.
//!
//! The step keeps π, the net's probability of the executed action at the
//! sampling temperature, as its behaviour log-probability (the replay must
//! reproduce it), and records μ = (1 − ε)·π + ε·1[a = R] beside it with the
//! proposer. Every head the layer answers is covered: priority and the other
//! single choices (targets, modes, mulligans), subsets (attackers, several
//! targets, cards), blocker pairs and numbers. A number has no density: π is
//! the point mass at the net's answer, and its step claims no probability.
//!
//! The layer's answer is an action taken, never a label.

use baylee_core::ids::ObjectId;
use baylee_engine::choice::Pending;
use baylee_train_ops::exploration::{
    self, GameExploration, NET, RULES, RuleProposal, SEED_SALT, StepExploration,
};
use baylee_view::PlayerView;

use super::policy::{
    bernoulli_logprob, categorical_logprob, decode_target, tactical_pair_logits, tactical_scalar,
    tactical_subset_logits,
};
use super::{TacticRules4, tactics, valid_raw_temperature};
use crate::deckgen::Rng;
use crate::features4::{
    EncodedCandidates4, HeadKind4, TeacherTarget4, denormalize_scalar_choice,
    normalize_scalar_choice, pair_free_mask_of, sample_pairs_constrained,
    sample_subset_constrained, subset_free_mask_of,
};

/// The learner's explorer: the groups it proposes with, ε, and its stream.
#[derive(Clone, Debug)]
pub struct RuleExplorer4 {
    rules: TacticRules4,
    epsilon: f64,
    seed: u64,
    rng: Rng,
    draws: u32,
    played: u32,
    last: Option<StepExploration>,
}

/// What the explorer reads of one decision.
pub(super) struct ExploreStep<'a> {
    pub(super) cands: &'a EncodedCandidates4,
    pub(super) view: &'a PlayerView,
    pub(super) pending: &'a Pending,
    pub(super) cand_logits: &'a [f32],
    pub(super) pair_logits: &'a [f32],
    pub(super) scalar_pred: f32,
    pub(super) active_source: Option<(ObjectId, Option<u32>)>,
    /// The net's sampling stream as it stood before its draw.
    pub(super) rng: &'a Rng,
    pub(super) temperature: f32,
    /// Whether the net plays its raw policy.
    pub(super) raw: bool,
}

/// The layer's answer, in the target's terms.
#[derive(Clone, Debug, PartialEq, Eq)]
enum RuleAnswer {
    /// A candidate's index.
    Single(usize),
    /// A subset or pair mask.
    Mask(Vec<u8>),
    /// A number.
    Scalar(u32),
}

impl RuleExplorer4 {
    /// An explorer proposing the answers of the layer with `rules`, played
    /// with probability `epsilon` where they differ from the net's.
    ///
    /// # Errors
    /// An ε outside `(0, 1)`.
    pub fn new(rules: TacticRules4, epsilon: f64) -> anyhow::Result<Self> {
        exploration::validate_epsilon(epsilon)?;
        Ok(Self {
            rules,
            epsilon,
            seed: SEED_SALT,
            rng: Rng::new(SEED_SALT),
            draws: 0,
            played: 0,
            last: None,
        })
    }

    /// Seeds a game's stream from the learner's policy seed
    /// ([`exploration::SEED_RULE`]), independent of the net's own stream.
    pub fn reseed(&mut self, policy_seed: u64) {
        self.seed = policy_seed ^ SEED_SALT;
        self.rng = Rng::new(self.seed);
        self.draws = 0;
        self.played = 0;
        self.last = None;
    }

    /// The game's stream so far: its seed, coins drawn and answers played.
    #[must_use]
    pub const fn game(&self) -> GameExploration {
        GameExploration {
            seed: self.seed,
            draws: self.draws,
            played: self.played,
        }
    }

    /// The latest decision's record, taken.
    pub const fn take_last(&mut self) -> Option<StepExploration> {
        self.last.take()
    }

    /// Forgets the latest record: a decision that never reaches the sampler.
    pub(super) fn clear(&mut self) {
        self.last = None;
    }

    /// Explores one decision whose raw sample is `target` with
    /// log-probability `logprob`: plays the layer's answer in its place when
    /// they differ and the coin says so, and returns the executed action's
    /// log-probability under the net.
    ///
    /// # Errors
    /// A decision the raw policy does not sample stochastically.
    pub(super) fn explore(
        &mut self,
        step: &ExploreStep<'_>,
        target: &mut TeacherTarget4,
        logprob: f32,
    ) -> anyhow::Result<f32> {
        self.last = None;
        anyhow::ensure!(
            step.raw && step.temperature > 0.35 && valid_raw_temperature(step.temperature),
            "rule exploration needs the raw policy sampling above 0.35"
        );
        let mut rules = self.rules;
        rules.take_fired();
        let answer = rule_answer(&mut rules, step).filter(|answer| playable(step, answer));
        let groups: Vec<&str> = rules
            .take_fired()
            .into_iter()
            .filter(|&(_, count)| count > 0)
            .map(|(name, _)| name)
            .collect();
        let rule = match &answer {
            None => RuleProposal::Absent,
            Some(answer) if sampled(answer, target, step.cands) => RuleProposal::Same,
            Some(_) => RuleProposal::Differs,
        };
        let draw = self.draws;
        self.draws += 1;
        let coin = self.rng.unit();
        let played = rule == RuleProposal::Differs && coin < self.epsilon;
        let mut logprob = logprob;
        if played && let Some(answer) = &answer {
            logprob = play(step, answer, target)?;
            self.played += 1;
        }
        let has_logprob = step.cands.head != HeadKind4::ScalarValue;
        let proposer = match groups.as_slice() {
            _ if !played => NET.to_owned(),
            [group] => format!("rule:{group}"),
            _ => RULES.to_owned(),
        };
        self.last = Some(StepExploration {
            draw,
            coin,
            rule,
            played,
            proposer,
            mixture_logprob: exploration::mixture_logprob(
                exploration::net_probability(has_logprob, logprob, played),
                self.epsilon,
                rule,
                played,
            ),
        });
        Ok(logprob)
    }
}

/// The guard `--choice-guard tactical` plays with: every part of the layer.
fn tactical_guard() -> super::ChoiceGuard4 {
    let mut guard = super::ChoiceGuard4::default();
    guard.tactical = super::TacticalParts {
        priority: true,
        single: true,
        subset: true,
        pair: true,
        scalar: true,
    };
    guard
}

/// What the layer with `rules` answers at temperature 0 on the step's
/// logits, read on a copy of the net's stream.
fn rule_answer(rules: &mut TacticRules4, step: &ExploreStep<'_>) -> Option<RuleAnswer> {
    let mut rng = step.rng.clone();
    let cands = step.cands;
    Some(match cands.head {
        HeadKind4::SingleChoice => {
            let n = cands.candidates.len();
            let (chosen, _) = tactics::rules::single_choice(
                rules,
                cands,
                step.cand_logits,
                step.view,
                step.pending,
                step.active_source,
                0.0,
                &mut rng,
            );
            let chosen = tactics::guard::review(
                &mut tactical_guard(),
                cands,
                step.cand_logits.get(..n)?,
                step.view,
                step.pending,
                chosen,
            );
            RuleAnswer::Single(chosen)
        }
        HeadKind4::SubsetPick => {
            let n = cands.candidates.len();
            let adjusted = tactical_subset_logits(
                rules,
                cands,
                step.cand_logits.get(..n)?,
                step.view,
                step.pending,
                step.active_source,
            );
            let (mask, ..) =
                sample_subset_constrained(&adjusted, cands, step.pending, 0.0, &mut rng);
            RuleAnswer::Mask(mask)
        }
        HeadKind4::PairPick => {
            let n = cands.pairs.len();
            let adjusted = tactical_pair_logits(
                rules,
                cands,
                step.pair_logits.get(..n)?,
                step.view,
                step.pending,
            );
            let (mask, ..) = sample_pairs_constrained(&adjusted, cands, 0.0, &mut rng);
            RuleAnswer::Mask(mask)
        }
        HeadKind4::ScalarValue => {
            let pred = tactical_scalar(step.pending, step.view, step.scalar_pred);
            RuleAnswer::Scalar(denormalize_scalar_choice(
                pred,
                cands.min_choices,
                cands.max_choices,
            ))
        }
    })
}

/// Whether the net's sample `target` is the layer's `answer`.
fn sampled(answer: &RuleAnswer, target: &TeacherTarget4, cands: &EncodedCandidates4) -> bool {
    match answer {
        RuleAnswer::Single(index) => usize::from(target.single_idx) == *index,
        RuleAnswer::Mask(mask) => target.multi_mask == *mask,
        RuleAnswer::Scalar(value) => {
            target.scalar_norm.to_bits()
                == normalize_scalar_choice(*value, cands.min_choices, cands.max_choices).to_bits()
        }
    }
}

/// The policy target of the layer's answer, with the conditional bits under
/// which the raw sampler would draw it; `None` outside the sampler's support.
fn rule_target(step: &ExploreStep<'_>, answer: &RuleAnswer) -> Option<TeacherTarget4> {
    let cands = step.cands;
    let mut target = TeacherTarget4::default();
    match answer {
        RuleAnswer::Single(index) => {
            if *index >= cands.candidates.len() {
                return None;
            }
            target.single_idx = u16::try_from(*index).ok()?;
        }
        RuleAnswer::Mask(mask) => {
            let free_mask = match cands.head {
                HeadKind4::SubsetPick => subset_free_mask_of(cands, step.pending, mask)?,
                HeadKind4::PairPick => pair_free_mask_of(cands, mask)?,
                HeadKind4::SingleChoice | HeadKind4::ScalarValue => return None,
            };
            target.multi_mask.clone_from(mask);
            target.free_mask = free_mask;
            // The samplers visit in the canonical candidate order.
            target.pick_sequence = (0..mask.len()).map(|i| i as u16).collect();
        }
        RuleAnswer::Scalar(value) => {
            target.scalar_norm =
                normalize_scalar_choice(*value, cands.min_choices, cands.max_choices);
        }
    }
    Some(target)
}

/// Whether the layer's answer can be played as the raw policy's own: in the
/// sampler's support, decoding to exactly itself, and legal.
fn playable(step: &ExploreStep<'_>, answer: &RuleAnswer) -> bool {
    rule_target(step, answer).is_some_and(|target| {
        decode_target(
            step.cands,
            step.view,
            step.pending,
            &target,
            step.active_source,
            true,
        )
        .is_ok()
    })
}

/// Puts the layer's answer in `target` and returns its log-probability
/// under the net.
fn play(
    step: &ExploreStep<'_>,
    answer: &RuleAnswer,
    target: &mut TeacherTarget4,
) -> anyhow::Result<f32> {
    let rule = rule_target(step, answer)
        .ok_or_else(|| anyhow::anyhow!("a playable layer answer has no target"))?;
    let cands = step.cands;
    let logprob = match answer {
        RuleAnswer::Single(index) => categorical_logprob(
            &step.cand_logits[..cands.candidates.len()],
            *index,
            step.temperature,
        ),
        RuleAnswer::Mask(mask) => {
            let raw = match cands.head {
                HeadKind4::PairPick => &step.pair_logits[..cands.pairs.len()],
                _ => &step.cand_logits[..cands.candidates.len()],
            };
            bernoulli_logprob(raw, mask, &rule.free_mask, step.temperature)
        }
        RuleAnswer::Scalar(_) => 0.0,
    };
    *target = rule;
    Ok(logprob)
}
