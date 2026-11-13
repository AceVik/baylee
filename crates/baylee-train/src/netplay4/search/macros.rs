//! Macro candidates at priority ([`SearchConfig4::with_macros`], v5).
//!
//! The policy's ranking is a poor list of what to compare. A spell is on the
//! engine's offer only with its mana floating, so "cast this" starts at one
//! of several equivalent taps, and the next `K - 1` answers by raw logit are
//! mostly those taps. Here the candidates are read off the offer instead:
//! the policy's own answer, then a pass, each spell the planner can pay for
//! (its taps and the cast as one candidate, a [`Script4`]), each spell
//! castable as things stand, each land and each ability that is not a mana
//! ability, one per card. Every one of them is read from the seat's view and
//! the question's offer, so two tables that differ only in what the seat
//! cannot see compare the same candidates.
//!
//! A root is the seat's priority at the start of a chain (no mana floating,
//! no spell being cast, no payment owed) where the policy acts, or in the
//! seat's own main phase with an empty stack, or with another player's
//! object on top of the stack. Every other priority is the policy's.
//!
//! [`SearchConfig4::with_macros`]: super::SearchConfig4::with_macros

use baylee_core::ids::ObjectId;
use baylee_engine::choice::LegalActions;
use baylee_engine::engine::ForkRefusal;
use baylee_view::{CardIdentity, Phase};

use super::rollout::{follow_script, world};
use super::{
    Candidate4, ForkSource4, Pending, PlayerAction, PlayerView, Ranked4, RootDecision4, Script4,
    SearchPlayer4, SearchSkip4, ViewContext4,
};
use crate::netplay4::menu::reachable;
use crate::netplay4::tactics::{is_mana_ability_entry, lookup_card_identity};

/// Whether the seat is at the start of a chain: nothing floating, no spell
/// being cast, no payment owed.
fn chain_start(view: &PlayerView) -> bool {
    view.owed.is_none()
        && view.casting.is_none()
        && view.announced_cast.is_none()
        && view
            .seats
            .iter()
            .filter(|seat| seat.player == view.seat)
            .all(|seat| seat.mana_pool.total() == 0)
}

/// Why `pending` is not a root of a search with macro candidates.
pub(super) fn root_skip(
    view: &PlayerView,
    pending: &Pending,
    decision: &RootDecision4,
) -> Option<SearchSkip4> {
    if !matches!(pending, Pending::Priority { .. }) {
        return Some(SearchSkip4::NotPriority);
    }
    if decision.ranking.is_none() {
        return Some(SearchSkip4::NotPolicy);
    }
    macro_root_skip(view, pending, &decision.answer.action)
}

/// Why `pending`, which the policy answers `own`, is not a macro root: the
/// rule of the search's roots without the search's own conditions on the
/// answer (a ranking to compare), as the search student reads it at the
/// table.
pub(in crate::netplay4) fn macro_root_skip(
    view: &PlayerView,
    pending: &Pending,
    own: &PlayerAction,
) -> Option<SearchSkip4> {
    if !matches!(pending, Pending::Priority { .. }) {
        return Some(SearchSkip4::NotPriority);
    }
    if !chain_start(view) {
        return Some(SearchSkip4::MidChain);
    }
    let acts = !matches!(own, PlayerAction::PassPriority);
    let own_main = view.active == view.seat
        && matches!(view.phase, Phase::FirstMain | Phase::SecondMain)
        && view.stack.is_empty();
    let must_answer = view
        .stack
        .last()
        .is_some_and(|top| top.controller != view.seat);
    (!(acts || own_main || must_answer)).then_some(SearchSkip4::Phase)
}

/// The policy's answer as a candidate, with its place in the ranking where
/// it has one.
fn ranked(decision: &RootDecision4, action: PlayerAction) -> Ranked4 {
    decision
        .ranking
        .as_deref()
        .unwrap_or_default()
        .iter()
        .find(|ranked| ranked.action == action)
        .cloned()
        .unwrap_or(Ranked4 {
            index: usize::from(u16::MAX),
            action,
            logit: f32::NEG_INFINITY,
        })
}

/// The first object of each card among `ids`, in order: two copies of a
/// card are one candidate.
fn one_per_card(view: &PlayerView, ids: impl IntoIterator<Item = ObjectId>) -> Vec<ObjectId> {
    let mut seen: Vec<CardIdentity> = Vec::new();
    let mut out = Vec::new();
    for id in ids {
        match lookup_card_identity(view, id) {
            Some(card) if seen.contains(&card) => {}
            Some(card) => {
                seen.push(card);
                out.push(id);
            }
            None => out.push(id),
        }
    }
    out
}

/// The abilities of `legal` that are not mana abilities, one per card and
/// ability.
fn abilities(view: &PlayerView, legal: &LegalActions) -> Vec<PlayerAction> {
    let mut seen: Vec<(Option<CardIdentity>, u32)> = Vec::new();
    let mut out = Vec::new();
    for &(source, ability_index) in &legal.abilities {
        if is_mana_ability_entry(view, source, ability_index) {
            continue;
        }
        let key = (lookup_card_identity(view, source), ability_index);
        if key.0.is_some() && seen.contains(&key) {
            continue;
        }
        seen.push(key);
        out.push(PlayerAction::ActivateAbility {
            source,
            ability_index,
        });
    }
    out
}

/// The candidates of the root `pending`, the policy's own first, at most
/// `limit` in all: see the module's head for the order.
pub(in crate::netplay4) fn candidates(
    view: &PlayerView,
    pending: &Pending,
    decision: &RootDecision4,
    limit: usize,
) -> Vec<Candidate4> {
    let own = &decision.answer.action;
    let mut out = vec![Candidate4::plain(ranked(decision, own.clone()))];
    let Pending::Priority { legal, .. } = pending else {
        return out;
    };
    let single = |action: PlayerAction, out: &mut Vec<Candidate4>| {
        if action != *own && pending.answer_fault(&action).is_none() {
            out.push(Candidate4::plain(ranked(decision, action)));
        }
    };
    if legal.can_pass {
        single(PlayerAction::PassPriority, &mut out);
    }
    let planned = reachable(view, legal);
    for card in one_per_card(view, planned.iter().map(|(card, _)| *card)) {
        let Some((_, plan)) = planned.iter().find(|(id, _)| *id == card) else {
            continue;
        };
        if let Some((first, script)) = Script4::begin(view, plan, PlayerAction::CastSpell { card })
        {
            out.push(Candidate4 {
                root: ranked(decision, first),
                target: None,
                script: Some(script),
            });
        }
    }
    for card in one_per_card(view, legal.castable.iter().copied()) {
        single(PlayerAction::CastSpell { card }, &mut out);
    }
    for card in one_per_card(view, legal.lands.iter().copied()) {
        single(PlayerAction::PlayLand { card }, &mut out);
    }
    for action in abilities(view, legal) {
        single(action, &mut out);
    }
    out.truncate(limit.max(1));
    out
}

/// The candidates of the root `pending` that the table takes in the world
/// forked from `seed`, the policy's own first: a planned cast whose taps or
/// cast the world refuses is dropped there, and so never refuses a round.
/// What a candidate needs is the seat's own (its lands, its hand), so the
/// first world answers for every world.
///
/// # Errors
/// The table's refusal to fork.
pub(super) fn playable<T: ForkSource4>(
    table: &T,
    context: ViewContext4,
    view: &PlayerView,
    pending: &Pending,
    decision: &RootDecision4,
    limit: usize,
    seed: u64,
) -> Result<Vec<Candidate4>, ForkRefusal> {
    let seat = view.seat;
    let mut out = Vec::new();
    for (at, candidate) in candidates(view, pending, decision, limit)
        .into_iter()
        .enumerate()
    {
        // The policy's own answer is the table's to refuse, as in play.
        if at > 0 {
            let mut probe = world(table, seat, seed)?;
            if probe
                .act_slice(seat, candidate.root.action.clone(), 0)
                .is_err()
            {
                continue;
            }
            if let Some(mut script) = candidate.script.clone()
                && !follow_script(Unobserved, &mut probe, context, seat, &mut script)
                    .unwrap_or(false)
            {
                continue;
            }
        }
        out.push(candidate);
    }
    Ok(out)
}

/// Nobody's bookkeeping: a script followed in a probe world, where no
/// player plays on.
pub(super) struct Unobserved;

/// What following a script asks of the player whose seat it is.
pub(super) trait ScriptWitness {
    /// Takes in the seat's view before its question, as before any answer.
    ///
    /// # Errors
    /// When the player cannot observe.
    fn observe(&mut self, view: &PlayerView) -> anyhow::Result<()>;
    /// Notes `action` as the seat's answer to `pending`.
    fn answer(&mut self, view: &PlayerView, pending: &Pending, action: &PlayerAction);
}

impl ScriptWitness for Unobserved {
    fn observe(&mut self, _: &PlayerView) -> anyhow::Result<()> {
        Ok(())
    }
    fn answer(&mut self, _: &PlayerView, _: &Pending, _: &PlayerAction) {}
}

/// A player and the table's teams: it observes and answers a script's
/// questions as its own choices (decide, then finish with the answer).
pub(super) struct Playing<'a, P> {
    pub(super) player: &'a mut P,
    pub(super) teams: &'a [Option<u8>],
}

impl<P: SearchPlayer4> ScriptWitness for Playing<'_, P> {
    fn observe(&mut self, view: &PlayerView) -> anyhow::Result<()> {
        self.player.observe(view)
    }
    fn answer(&mut self, view: &PlayerView, pending: &Pending, action: &PlayerAction) {
        let decision = self.player.decide(view, pending, self.teams);
        self.player.conclude(view, &decision, action);
    }
}
