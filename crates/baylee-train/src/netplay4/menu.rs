//! `eval4 --menu-payable`: a dev-only arm, off by default.
//!
//! The spells of `legal.payable` the planner can pay join the priority menu at
//! run time, as `features7::encode_candidates7` adds them and as the language
//! model's seat offers them (`baylee_seat::wake::reachable`). When the net
//! answers with one, the planner taps for it step by step and the spell is
//! then cast from the pool, as a seat's `Act::Taps` does. The net's own menu
//! stays as it is, so the arm only ever adds answers.
//!
//! The net never saw these candidates: in training a spell is on the menu only
//! with its mana floating. [`MenuStats4`] says what a frozen net makes of them.
//! A measurement and nothing else: `eval4` refuses it with collection, search
//! and shadow labels, and no such run may become training data.

use baylee_client_core::manaplan::Plan;
use baylee_core::ids::ObjectId;
use baylee_engine::choice::{CastModeKind, LegalActions, Pending, PlayerAction};
use baylee_view::{Phase, PlayerView};

use super::script::{Script4, ScriptAnswer4};
use super::tactics::{lookup_card_identity, lookup_spell_cost};
use crate::features4::{EncodedCandidates4, MAX_CANDIDATES};

/// What a run's net did with the options it never saw.
#[derive(Clone, Copy, Debug, Default, serde::Serialize)]
pub struct MenuStats4 {
    /// Priority menus that held at least one added option.
    pub menus: u64,
    /// Added options offered, over all menus.
    pub options: u64,
    /// Menus whose best logit was an added option.
    pub topped: u64,
    /// Menus whose best added option beat passing.
    pub beat_pass: u64,
    /// Menus by the added options' share of the softmax at `T = 1`, in tenths.
    pub mass: [u64; 10],
    /// Answers that chose an added option.
    pub chosen: u64,
    /// Of those, on the other seat's turn.
    pub chosen_their_turn: u64,
    /// Of those, in the seat's own main phases.
    pub chosen_own_main: u64,
    /// Taps the planner made.
    pub taps: u64,
    /// Spells cast once the taps were made.
    pub cast: u64,
    /// Plans ended early: a tap the table would not take, or the moment passed.
    pub broken: u64,
    /// Plans whose taps all went through and whose spell was not castable.
    pub not_castable: u64,
}

impl MenuStats4 {
    /// Adds `other` to these counts.
    pub fn add(&mut self, other: &Self) {
        self.menus += other.menus;
        self.options += other.options;
        self.topped += other.topped;
        self.beat_pass += other.beat_pass;
        for (mine, theirs) in self.mass.iter_mut().zip(&other.mass) {
            *mine += theirs;
        }
        self.chosen += other.chosen;
        self.chosen_their_turn += other.chosen_their_turn;
        self.chosen_own_main += other.chosen_own_main;
        self.taps += other.taps;
        self.cast += other.cast;
        self.broken += other.broken;
        self.not_castable += other.not_castable;
    }
}

/// The arm's state on one net.
#[derive(Clone, Debug, Default)]
pub struct PayableMenu4 {
    /// Added to each added option's logit, a net that never saw them being
    /// free to read them as anything.
    bias: f32,
    offered: Vec<(ObjectId, Plan)>,
    first_added: usize,
    plan: Option<Script4>,
    stats: MenuStats4,
}

/// The spells of `legal.payable` the planner has taps for, each with its plan.
/// A plan with nothing to tap is no reach: the mana is floating, so what keeps
/// the spell out of `castable` is not mana.
pub(super) fn reachable(view: &PlayerView, legal: &LegalActions) -> Vec<(ObjectId, Plan)> {
    if view.owed.is_some() {
        return Vec::new();
    }
    let actor = view.decision_player.unwrap_or(view.seat);
    let Some(pool) = view.seat(actor).map(|seat| seat.mana_pool) else {
        return Vec::new();
    };
    legal
        .payable
        .iter()
        .copied()
        .filter(|id| !legal.castable.contains(id))
        .filter_map(|id| {
            let card = lookup_card_identity(view, id)?;
            if !baylee_ai::spell_can_be_cast_now(view, legal, id, card) {
                return None;
            }
            let (printed, _) = lookup_spell_cost(view, id)?;
            // An `{X}` spell at its least useful, the tactical layer's reading.
            let printed = printed.with_x(u32::from(printed.has_variable()));
            let price =
                baylee_ai::spell_price(view, id, Some(card), legal, CastModeKind::Normal, printed);
            let plan = baylee_ai::plan_for_spell(&price, &pool, view, legal, id)?;
            (!plan.steps.is_empty()).then_some((id, plan))
        })
        .collect()
}

impl PayableMenu4 {
    /// The arm with `bias` on the added options' logits.
    #[must_use]
    pub fn new(bias: f32) -> Self {
        Self {
            bias,
            ..Self::default()
        }
    }

    /// Clears what a game left behind; the counts stay until taken.
    pub(super) fn reset(&mut self) {
        self.offered.clear();
        self.plan = None;
    }

    /// The counts since this was last asked.
    pub fn take_stats(&mut self) -> MenuStats4 {
        std::mem::take(&mut self.stats)
    }

    /// Appends the reachable spells to a priority menu, encoded by `encode`
    /// exactly as the same spells are with their mana floating.
    pub(super) fn extend(
        &mut self,
        cands: &mut EncodedCandidates4,
        view: &PlayerView,
        pending: &Pending,
        encode: impl FnOnce(&Pending) -> EncodedCandidates4,
    ) {
        self.offered.clear();
        self.first_added = cands.candidates.len();
        let Pending::Priority { player, legal } = pending else {
            return;
        };
        self.offered = reachable(view, legal);
        self.offered
            .truncate(MAX_CANDIDATES.saturating_sub(cands.candidates.len()));
        if self.offered.is_empty() {
            return;
        }
        let mut announced = legal.clone();
        announced.castable = self.offered.iter().map(|(id, _)| *id).collect();
        let extra = encode(&Pending::Priority {
            player: *player,
            legal: announced,
        });
        cands.candidates.extend(
            extra
                .candidates
                .into_iter()
                .filter(|cand| matches!(cand.action, Some(PlayerAction::CastSpell { .. }))),
        );
    }

    /// Applies the bias and counts how the net scored the added options.
    pub(super) fn weigh(&mut self, logits: &mut [f32], cands: &EncodedCandidates4) {
        let n = cands.candidates.len();
        if self.offered.is_empty() || self.first_added >= n || logits.len() < n {
            return;
        }
        for logit in &mut logits[self.first_added..n] {
            *logit += self.bias;
        }
        let logits = &logits[..n];
        let top = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let added = logits[self.first_added..]
            .iter()
            .copied()
            .fold(f32::NEG_INFINITY, f32::max);
        let total: f32 = logits.iter().map(|logit| (logit - top).exp()).sum();
        let share: f32 = logits[self.first_added..]
            .iter()
            .map(|logit| (logit - top).exp())
            .sum::<f32>()
            / total;
        let pass = cands
            .candidates
            .iter()
            .position(|cand| matches!(cand.action, Some(PlayerAction::PassPriority)))
            .map_or(f32::NEG_INFINITY, |index| logits[index]);
        self.stats.menus += 1;
        self.stats.options += (n - self.first_added) as u64;
        self.stats.topped += u64::from(added >= top);
        self.stats.beat_pass += u64::from(added > pass);
        self.stats.mass[((share * 10.0) as usize).min(9)] += 1;
    }

    /// The first tap of the plan for `action`, when it casts an added option.
    pub(super) fn begin(
        &mut self,
        view: &PlayerView,
        action: &PlayerAction,
    ) -> Option<PlayerAction> {
        let PlayerAction::CastSpell { card } = action else {
            return None;
        };
        let (_, plan) = self.offered.iter().find(|(id, _)| id == card)?;
        let (first, script) = Script4::begin(view, plan, action.clone())?;
        let actor = view.decision_player.unwrap_or(view.seat);
        self.stats.chosen += 1;
        if view.active != actor {
            self.stats.chosen_their_turn += 1;
        } else if matches!(view.phase, Phase::FirstMain | Phase::SecondMain) {
            self.stats.chosen_own_main += 1;
        }
        self.stats.taps += 1;
        self.plan = Some(script);
        Some(first)
    }

    /// The plan's next answer, when one stands and this question is its own.
    /// Any other question stays the net's.
    pub(super) fn next(&mut self, view: &PlayerView, pending: &Pending) -> Option<PlayerAction> {
        let answer = self.plan.as_mut()?.next(view, pending);
        if answer.ends() {
            self.plan = None;
        }
        match answer {
            ScriptAnswer4::Tap(action) => {
                self.stats.taps += 1;
                Some(action)
            }
            ScriptAnswer4::Color(action) => Some(action),
            ScriptAnswer4::Then(action) => {
                self.stats.cast += 1;
                Some(action)
            }
            ScriptAnswer4::Broken => {
                self.stats.broken += 1;
                None
            }
            ScriptAnswer4::NotOffered => {
                self.stats.not_castable += 1;
                None
            }
            ScriptAnswer4::Other => None,
        }
    }
}
