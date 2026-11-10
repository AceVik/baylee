//! `eval4 --choice-guard`: dev-only ceilings, off by default.
//!
//! Each arm is one rule a reader can state about a raw net's answer, turned
//! into an override, so that a paired evaluation says what the fault costs.
//! They measure and nothing else: `eval4` refuses them with any collection,
//! search, shadow or temperature, and the run says so in `run.json`.

use baylee_core::ids::ObjectId;
use baylee_core::types::TypeSet;
use baylee_engine::choice::{LegalActions, Pending, PlayerAction};
use baylee_view::{Phase, PlayerView, Step};

use super::{
    AbilityDef, has_reachable_mana_sink, is_counterspell_effects, is_mana_action,
    is_spell_worth_casting_now_inner, is_x_spell_ready_from_castable, lookup_card_identity,
    reading, redirect_mana_source_for_colors,
};
use crate::features4::EncodedCandidates4;

/// One rule of the instrument.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuardArm {
    /// No casting a spell the tactical layer reads as doing nothing now.
    Cast,
    /// No tapping for mana while nothing reachable could spend it.
    Tap,
    /// No ending the own second main with a proactive spell paid for by what
    /// is untapped: tap for it and cast it.
    Spend,
    /// A first seven of 0, 1, 6 or 7 lands goes back.
    Mulligan,
    /// On the other seat's turn a usable instant stops being held: a
    /// counterspell answers their spell, anything else goes when they attack
    /// or at their end step.
    React,
}

const ARMS: usize = 5;

/// Which heads answer as `PolicyMode4::Tactical` does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "One switch per head of the net, each measured alone"
)]
pub struct TacticalParts {
    /// Single choices at priority.
    pub priority: bool,
    /// Every other single choice: targets, colours, modes, cards.
    pub single: bool,
    /// Subset picks: attackers, and cards chosen several at a time.
    pub subset: bool,
    /// Pair picks: blocks, and attackers with several defenders.
    pub pair: bool,
    /// Numbers.
    pub scalar: bool,
}

/// The instrument's state on one net.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ChoiceGuard4 {
    /// Parts of the tactical layer, as `PolicyMode4::Tactical` plays them, while
    /// the run stays raw for comparison.
    pub tactical: TacticalParts,
    /// Counts `P(pass)` by window; changes no answer.
    pub probing: bool,
    /// `eval4 --dev-free-temperature`: a raw net may sample at any temperature.
    /// Below sampler 2's boundary the single choices are sampled and subset and
    /// pair picks stay greedy with a little noise, which is why it is a dev flag.
    pub free_temperature: bool,
    arms: [bool; ARMS],
    fired: [u64; ARMS],
    probe: [[u64; 10]; 3],
}

impl ChoiceGuard4 {
    /// Turns an arm on.
    pub const fn enable(&mut self, arm: GuardArm) {
        self.arms[arm as usize] = true;
    }

    const fn on(&self, arm: GuardArm) -> bool {
        self.arms[arm as usize]
    }

    const fn count(&mut self, arm: GuardArm) {
        self.fired[arm as usize] += 1;
    }

    /// Answers changed since this was last asked, by arm: cast, tap, spend,
    /// mulligan, react.
    /// After them the probe's three windows, ten tenths each.
    pub fn take_fired(&mut self) -> Vec<u64> {
        let mut all = self.fired.to_vec();
        self.fired = [0; ARMS];
        for window in &mut self.probe {
            all.extend_from_slice(window);
            *window = [0; 10];
        }
        all
    }
}

/// The best candidate `allowed` lets through, by the net's own logits.
fn best(
    cands: &EncodedCandidates4,
    logits: &[f32],
    allowed: impl Fn(&PlayerAction) -> bool,
) -> Option<usize> {
    cands
        .candidates
        .iter()
        .enumerate()
        .filter(|(_, cand)| cand.action.as_ref().is_some_and(&allowed))
        .map(|(index, _)| index)
        .max_by(|a, b| logits[*a].total_cmp(&logits[*b]))
}

/// A spell whose casting reads as doing nothing now.
fn pointless(view: &PlayerView, action: &PlayerAction) -> bool {
    matches!(action, PlayerAction::CastSpell { card }
        if !is_spell_worth_casting_now_inner(view, *card))
}

/// The proactive spells worth casting that the seat can pay for.
fn owed_spells(view: &PlayerView, legal: &LegalActions) -> Vec<ObjectId> {
    let instant = |id: ObjectId| {
        view.hand
            .iter()
            .find(|card| card.id == id)
            .is_none_or(|card| card.types.contains(TypeSet::INSTANT))
    };
    reading::reachable_spells(view, legal)
        .into_iter()
        .filter(|id| !instant(*id) && is_spell_worth_casting_now_inner(view, *id))
        .collect()
}

/// The cast of the best of `spells` the pool pays, else the tap towards them.
pub(super) fn cast_or_tap(
    cands: &EncodedCandidates4,
    logits: &[f32],
    view: &PlayerView,
    legal: &LegalActions,
    spells: &[ObjectId],
) -> Option<usize> {
    let actor = view.decision_player.unwrap_or(view.seat);
    let pool = view
        .seat(actor)
        .map_or(0, |seat| u32::try_from(seat.mana_pool.total()).unwrap_or(0));
    let cast = best(cands, logits, |action| {
        matches!(action, PlayerAction::CastSpell { card }
            if spells.contains(card) && is_x_spell_ready_from_castable(view, *card, pool))
    });
    cast.or_else(|| {
        best(cands, logits, |action| is_mana_action(view, action))
            .map(|tap| redirect_mana_source_for_colors(cands, logits, view, legal, tap))
    })
}

/// `Spend`: what a pass at the end of the own second main is turned to.
pub(super) fn spend(
    cands: &EncodedCandidates4,
    logits: &[f32],
    view: &PlayerView,
    legal: &LegalActions,
) -> Option<usize> {
    let actor = view.decision_player.unwrap_or(view.seat);
    if view.active != actor
        || view.phase != Phase::SecondMain
        || !view.stack.is_empty()
        || view.owed.is_some()
    {
        return None;
    }
    let spells = owed_spells(view, legal);
    if spells.is_empty() {
        return None;
    }
    cast_or_tap(cands, logits, view, legal, &spells)
}

/// The other seat's window this decision stands in (0 their spell on the
/// stack, 1 their attack, 2 their end step) and the instants that fit it.
pub(super) fn react_window(
    view: &PlayerView,
    legal: &LegalActions,
) -> Option<(usize, Vec<ObjectId>)> {
    let actor = view.decision_player.unwrap_or(view.seat);
    if view.active == actor || view.owed.is_some() {
        return None;
    }
    let theirs_on_stack = !view.stack.is_empty()
        && view
            .stack
            .iter()
            .all(|item| super::is_opponent(actor, item.controller));
    let their_attack = view.step == Step::DeclareAttackers && view.combat.is_active();
    let quiet = view.stack.is_empty() && (their_attack || view.step == Step::End);
    if !theirs_on_stack && !quiet {
        return None;
    }
    let instant = |id: ObjectId| {
        view.hand
            .iter()
            .any(|card| card.id == id && card.types.contains(TypeSet::INSTANT))
    };
    let counters = |id: ObjectId| {
        lookup_card_identity(view, id)
            .and_then(|identity| {
                baylee_cards::by_index(identity.index).map(|def| {
                    def.abilities_for_face(usize::from(identity.face))
                        .iter()
                        .any(|ability| {
                            matches!(ability, AbilityDef::Spell { effects, .. }
                                if is_counterspell_effects(effects))
                        })
                })
            })
            .unwrap_or(false)
    };
    let spells: Vec<ObjectId> = reading::reachable_spells(view, legal)
        .into_iter()
        .filter(|id| instant(*id) && is_spell_worth_casting_now_inner(view, *id))
        .filter(|id| counters(*id) == theirs_on_stack)
        .collect();
    if spells.is_empty() {
        return None;
    }
    let window = if theirs_on_stack {
        0
    } else if their_attack {
        1
    } else {
        2
    };
    Some((window, spells))
}

/// `React`: what a pass on the other seat's turn is turned to.
pub(super) fn react(
    cands: &EncodedCandidates4,
    logits: &[f32],
    view: &PlayerView,
    legal: &LegalActions,
) -> Option<usize> {
    let (_, spells) = react_window(view, legal)?;
    cast_or_tap(cands, logits, view, legal, &spells)
}

/// `Probe`: counts the net's probability of passing in each window, in tenths,
/// with no mana floating. Changes no answer.
fn probe(
    guard: &mut ChoiceGuard4,
    cands: &EncodedCandidates4,
    logits: &[f32],
    view: &PlayerView,
    legal: &LegalActions,
) {
    let actor = view.decision_player.unwrap_or(view.seat);
    if view
        .seat(actor)
        .is_none_or(|seat| seat.mana_pool.total() > 0)
    {
        return;
    }
    let Some((window, _)) = react_window(view, legal) else {
        return;
    };
    let Some(pass) = cands
        .candidates
        .iter()
        .position(|cand| matches!(cand.action, Some(PlayerAction::PassPriority)))
    else {
        return;
    };
    let top = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let total: f32 = logits.iter().map(|logit| (logit - top).exp()).sum();
    let chance = (logits[pass] - top).exp() / total;
    let tenth = ((chance * 10.0) as usize).min(9);
    guard.probe[window][tenth] += 1;
}

/// The answer the guard's arms leave of the net's `chosen`.
pub(in crate::netplay4) fn review(
    guard: &mut ChoiceGuard4,
    cands: &EncodedCandidates4,
    logits: &[f32],
    view: &PlayerView,
    pending: &Pending,
    chosen: usize,
) -> usize {
    let Some(action) = cands.candidates.get(chosen).and_then(|c| c.action.as_ref()) else {
        return chosen;
    };
    match pending {
        Pending::Priority { legal, .. } => {
            if guard.probing {
                probe(guard, cands, logits, view, legal);
            }
            let no_sink = guard.on(GuardArm::Tap) && !has_reachable_mana_sink(view, legal);
            let refuse_cast = guard.on(GuardArm::Cast);
            let refused = |action: &PlayerAction| {
                (refuse_cast && pointless(view, action))
                    || (no_sink && is_mana_action(view, action))
            };
            if guard.on(GuardArm::Spend)
                && matches!(action, PlayerAction::PassPriority)
                && let Some(better) = spend(cands, logits, view, legal)
            {
                guard.count(GuardArm::Spend);
                return better;
            }
            if guard.on(GuardArm::React)
                && matches!(action, PlayerAction::PassPriority)
                && let Some(better) = react(cands, logits, view, legal)
            {
                guard.count(GuardArm::React);
                return better;
            }
            if refused(action) {
                let arm = if is_mana_action(view, action) {
                    GuardArm::Tap
                } else {
                    GuardArm::Cast
                };
                if let Some(better) = best(cands, logits, |action| !refused(action)) {
                    guard.count(arm);
                    return better;
                }
            }
            chosen
        }
        Pending::Mulligan {
            taken, can_take, ..
        } if guard.on(GuardArm::Mulligan)
            && *taken == 0
            && *can_take
            && matches!(action, PlayerAction::MulliganKeep) =>
        {
            let lands = view
                .hand
                .iter()
                .filter(|card| card.types.contains(TypeSet::LAND))
                .count();
            if view.hand.len() == 7
                && !(2..=5).contains(&lands)
                && let Some(take) = best(cands, logits, |action| {
                    matches!(action, PlayerAction::MulliganTake)
                })
            {
                guard.count(GuardArm::Mulligan);
                return take;
            }
            chosen
        }
        _ => chosen,
    }
}
