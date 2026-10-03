//! Every answer the house gives is held to the question it answers.
//!
//! A question states every reason the engine refuses an answer to it
//! ([`Pending::answer_fault`], `docs/pending-constraints.md`), so an answer
//! that breaks one is a defect in whichever picker built it, never a
//! judgement call. The engine refuses it, and then a hosted seat has nothing
//! taken and waits on its clock, and a self-play game counts a refusal. So
//! the agent checks its own answer before it gives it.
//!
//! An answer that breaks its question is refitted to the nearest one inside
//! it ([`refit`]), and never silently: the agent logs a warning and counts
//! it ([`crate::HeuristicAgent::fallbacks`]), and the self-play sweeps
//! assert that the count stays at zero. The count is the finding. The refit
//! only keeps a real table moving until the picker is fixed.

use std::collections::BTreeSet;

use baylee_core::ids::{ObjectId, PlayerId};
use baylee_engine::choice::{Pending, PlayerAction, default_arrangement, timeout_answer};
use baylee_view::PlayerView;

/// The nearest answer to `proposal` that `pending` takes, or `None` when
/// none of the three below is taken.
///
/// The first of these the question takes:
/// 1. The proposal's own choices in the proposal's order, which is the
///    order its picker ranked them in: repeats and what the question did not
///    offer left out, cut to the question's maximum, and made up to its
///    minimum from what else it offers, an opponent's first ([`fitted`]).
/// 2. The answer that does nothing, where the question has one
///    ([`timeout_answer`]): pass, keep, attack or block with nothing,
///    decline.
/// 3. The least the question offers ([`least`]).
///
/// `hostile` says whether a seat is an opponent of the one answering.
pub(crate) fn refit(
    view: &PlayerView,
    pending: &Pending,
    proposal: &PlayerAction,
    hostile: &dyn Fn(PlayerId) -> bool,
) -> Option<PlayerAction> {
    let taken = |answer: &PlayerAction| pending.answer_fault(answer).is_none();
    fitted(view, pending, proposal, hostile)
        .filter(taken)
        .or_else(|| timeout_answer(pending).filter(taken))
        .or_else(|| least(view, pending, hostile).filter(taken))
}

/// The proposal's own choices held to the question's counts, for the
/// questions an answer names a list or a number to: `None` for every other
/// question, and for an answer of the wrong kind.
fn fitted(
    view: &PlayerView,
    pending: &Pending,
    proposal: &PlayerAction,
    hostile: &dyn Fn(PlayerId) -> bool,
) -> Option<PlayerAction> {
    let hand: Vec<ObjectId> = view.hand.iter().map(|card| card.id).collect();
    match (pending, proposal) {
        (
            Pending::ChooseTargets {
                options,
                player_options,
                min,
                max,
                ..
            },
            PlayerAction::ChooseTargets { objects, players },
        ) => Some(targets(
            view,
            (options, player_options),
            (objects, players),
            (*min, *max),
            hostile,
        )),
        (
            Pending::ChooseTargets {
                options,
                player_options,
                min,
                max,
                ..
            },
            PlayerAction::ChooseObjects { objects },
        ) => Some(targets(
            view,
            (options, player_options),
            (objects, &[]),
            (*min, *max),
            hostile,
        )),
        (
            Pending::ChooseCards {
                options,
                min,
                max,
                total,
                ..
            },
            PlayerAction::ChooseObjects { objects },
        ) => match total {
            // A total is a sum of the chosen cards' weights, which a list cut
            // to its counts does not keep: the fewest cards that reach it.
            Some(total) => crate::policy::reach_total(options, *min, *max, total),
            None => Some(kept(objects, options, options, (*min, *max))),
        }
        .map(|objects| PlayerAction::ChooseObjects { objects }),
        (Pending::LegendChoice { options, .. }, PlayerAction::ChooseObjects { objects }) => {
            Some(PlayerAction::ChooseObjects {
                objects: kept(objects, options, options, (1, 1)),
            })
        }
        // The question offers the seat's whole hand without listing it, and
        // the seat's own view lists it.
        (
            Pending::MulliganBottom { count, .. } | Pending::DiscardChoice { count, .. },
            PlayerAction::ChooseObjects { objects },
        ) => Some(PlayerAction::ChooseObjects {
            objects: kept(objects, &hand, &hand, (*count, *count)),
        }),
        (Pending::ChooseNumber { min, max, .. }, PlayerAction::ChooseNumber(n)) => {
            Some(PlayerAction::ChooseNumber((*n).clamp(*min, *max)))
        }
        _ => None,
    }
}

/// The least answer the question offers: [`fitted`] to an empty list, the
/// first option, the lowest number, the arrangement that places each card
/// in turn. `None` for a card name, which names no option.
fn least(
    view: &PlayerView,
    pending: &Pending,
    hostile: &dyn Fn(PlayerId) -> bool,
) -> Option<PlayerAction> {
    let nothing = PlayerAction::ChooseObjects {
        objects: Vec::new(),
    };
    match pending {
        Pending::ChooseDamageSource { .. }
        | Pending::ChooseDamageEffect { .. }
        | Pending::AllocatePrevention { .. } => crate::damage::answer(view, pending, hostile),
        Pending::ChooseTargets { .. }
        | Pending::ChooseCards { .. }
        | Pending::LegendChoice { .. }
        | Pending::MulliganBottom { .. }
        | Pending::DiscardChoice { .. } => fitted(view, pending, &nothing, hostile),
        Pending::ChooseNumber { min, .. } => Some(PlayerAction::ChooseNumber(*min)),
        Pending::ChooseSubtype { options, .. } => {
            options.first().map(|s| PlayerAction::ChooseSubtype(*s))
        }
        Pending::ChooseColor { options, .. } => {
            options.first().map(|c| PlayerAction::ChooseColor(*c))
        }
        Pending::ChoosePlayer { options, .. } => options
            .iter()
            .find(|p| hostile(**p))
            .or_else(|| options.first())
            .map(|p| PlayerAction::ChoosePlayer(*p)),
        Pending::ChooseCastMode { .. } | Pending::ChoosePile { .. } => {
            Some(PlayerAction::ChooseMode(0))
        }
        Pending::Arrange { cards, piles, .. } => {
            default_arrangement(cards, piles).map(|piles| PlayerAction::Arrange { piles })
        }
        Pending::YesNo { .. } => Some(PlayerAction::YesNo(false)),
        Pending::Mulligan { .. } => Some(PlayerAction::MulliganKeep),
        Pending::Priority { .. } => Some(PlayerAction::PassPriority),
        Pending::ChooseAttackers { .. } => Some(PlayerAction::DeclareAttackers {
            attackers: Vec::new(),
        }),
        Pending::ChooseBlockers { .. } => Some(PlayerAction::DeclareBlockers {
            blockers: Vec::new(),
        }),
        Pending::ChooseCardName { .. } | Pending::GameOver(_) => None,
    }
}

/// One instance of "target" (CR 115.3, 115.4): objects and players counted
/// together, the proposal's objects before its players, made up to `min`
/// with an opponent's objects, then opponents, then the rest.
fn targets(
    view: &PlayerView,
    (options, player_options): (&[ObjectId], &[PlayerId]),
    (objects, players): (&[ObjectId], &[PlayerId]),
    (min, max): (u32, u32),
    hostile: &dyn Fn(PlayerId) -> bool,
) -> PlayerAction {
    let (min, max) = (
        usize::try_from(min).unwrap_or(usize::MAX),
        usize::try_from(max).unwrap_or(usize::MAX),
    );
    let enemy = |id: &ObjectId| view.object(*id).is_none_or(|o| hostile(o.controller));
    let offered: BTreeSet<ObjectId> = options.iter().copied().collect();
    let seats: BTreeSet<PlayerId> = player_options.iter().copied().collect();
    let mut chosen: Vec<ObjectId> = Vec::new();
    let mut named: Vec<PlayerId> = Vec::new();
    let mut seen = BTreeSet::new();
    let mut seen_seats = BTreeSet::new();
    let count = |chosen: &Vec<ObjectId>, named: &Vec<PlayerId>| chosen.len() + named.len();
    for id in objects {
        if count(&chosen, &named) < max && offered.contains(id) && seen.insert(*id) {
            chosen.push(*id);
        }
    }
    for p in players {
        if count(&chosen, &named) < max && seats.contains(p) && seen_seats.insert(*p) {
            named.push(*p);
        }
    }
    let objects_by_side = options
        .iter()
        .filter(|id| enemy(id))
        .chain(options.iter().filter(|id| !enemy(id)));
    for id in objects_by_side {
        if count(&chosen, &named) >= min {
            break;
        }
        if seen.insert(*id) {
            chosen.push(*id);
        }
    }
    let seats_by_side = player_options
        .iter()
        .filter(|p| hostile(**p))
        .chain(player_options.iter().filter(|p| !hostile(**p)));
    for p in seats_by_side {
        if count(&chosen, &named) >= min {
            break;
        }
        if seen_seats.insert(*p) {
            named.push(*p);
        }
    }
    PlayerAction::ChooseTargets {
        objects: chosen,
        players: named,
    }
}

/// `chosen` in its own order, less repeats and what `offered` does not
/// hold, cut to `max` and made up to `min` from `fill`, in its order.
fn kept(
    chosen: &[ObjectId],
    offered: &[ObjectId],
    fill: &[ObjectId],
    (min, max): (u8, u8),
) -> Vec<ObjectId> {
    let (min, max) = (usize::from(min), usize::from(max));
    let offered: BTreeSet<ObjectId> = offered.iter().copied().collect();
    let mut seen = BTreeSet::new();
    let mut out: Vec<ObjectId> = Vec::new();
    for id in chosen {
        if out.len() < max && offered.contains(id) && seen.insert(*id) {
            out.push(*id);
        }
    }
    for id in fill {
        if out.len() >= min {
            break;
        }
        if offered.contains(id) && seen.insert(*id) {
            out.push(*id);
        }
    }
    out
}
