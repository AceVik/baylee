//! Raging River's piles ([`Effect::LeftRightPilesRestrictBlocks`]).
//!
//! "Each defending player divides all creatures without flying they control
//! into a 'left' pile and a 'right' pile. Then, for each attacking creature
//! you control, choose 'left' or 'right.' That creature can't be blocked
//! this combat except by creatures with flying and creatures in a pile with
//! the chosen label."
//!
//! The defending players are every opponent of the attacking player during
//! the combat phase (CR 802.2; the nonactive player of a two-player game,
//! CR 506.2), and they divide one after another in turn order from the
//! active player (CR 101.4), each naming their left pile
//! (`ChoicePrompt::LeftPile`; the rest are their right pile). The piles are
//! public: the creatures are on the battlefield. The controller then labels
//! each attacking creature they control, in declaration order, by choosing
//! one of the two piles (`Pending::ChoosePile` with `label`). A label is a
//! restriction on blocking that creature (CR 509.1b), held by the combat
//! (`CombatState::limit_blockers_to_pile`) and asked by `combat::can_block`:
//! a creature with flying, or one in that label's pile, may block it, and a
//! creature in no pile (one that entered since) may not.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary
use super::*;
use baylee_cards_dsl::KeywordSet;
use baylee_core::types::TypeSet;

/// The defending players, in turn order from the active player.
fn defending_players(state: &GameState) -> Vec<PlayerId> {
    let active = state.turn.active;
    let n = state.players.len();
    (1..n)
        .map(|k| {
            let seat = (usize::from(active.get()) + k) % n;
            PlayerId::new(u8::try_from(seat).unwrap_or(u8::MAX))
        })
        .filter(|&p| state.is_opponent(p, active) && !state.has_left(p))
        .collect()
}

/// The creatures without flying `player` controls, in battlefield order.
fn grounded(state: &GameState, player: PlayerId) -> Vec<ObjectId> {
    state
        .battlefield_view()
        .iter()
        .copied()
        .filter(|&id| {
            state.object(id).is_some_and(|o| {
                o.controller == player
                    && !o.status.contains(crate::object::Status::PHASED_OUT)
                    && o.characteristics().types.contains(TypeSet::CREATURE)
                    && !o.characteristics().keywords.contains(KeywordSet::FLYING)
            })
        })
        .collect()
}

/// Begins the effect: the first defending player with a creature to divide
/// is asked, or, with none, the labels are.
pub(super) fn begin(state: &mut GameState, res: &mut Resolution) -> Option<Pending> {
    divide_next(state, res, defending_players(state), Vec::new(), Vec::new())
}

fn divide_next(
    state: &mut GameState,
    res: &mut Resolution,
    mut rest: Vec<PlayerId>,
    left: Vec<ObjectId>,
    right: Vec<ObjectId>,
) -> Option<Pending> {
    while !rest.is_empty() {
        let asked = rest.remove(0);
        let options = grounded(state, asked);
        if options.is_empty() {
            continue;
        }
        let n = u8::try_from(options.len()).unwrap_or(u8::MAX);
        res.awaiting = Some(AwaitingOp::DivideLeftRight {
            asked,
            rest,
            left,
            right,
        });
        return Some(Pending::ChooseCards {
            player: asked,
            options,
            min: 0,
            max: n,
            prompt: ChoicePrompt::LeftPile,
            total: None,
        });
    }
    let attackers: Vec<ObjectId> = state
        .combat
        .attackers()
        .iter()
        .map(|a| a.creature)
        .filter(|&a| {
            state
                .object(a)
                .is_some_and(|o| o.controller == res.controller)
        })
        .collect();
    label_next(state, res, attackers, left, right)
}

fn label_next(
    state: &mut GameState,
    res: &mut Resolution,
    mut rest: Vec<ObjectId>,
    left: Vec<ObjectId>,
    right: Vec<ObjectId>,
) -> Option<Pending> {
    while !rest.is_empty() {
        let attacker = rest.remove(0);
        if !state.combat.is_attacking(attacker) {
            continue;
        }
        // Two empty piles are one choice: only fliers may block it either
        // way, so nobody is asked.
        if left.is_empty() && right.is_empty() {
            state.combat.limit_blockers_to_pile(attacker, Vec::new());
            continue;
        }
        let piles = vec![left.clone(), right.clone()];
        res.awaiting = Some(AwaitingOp::LabelAttacker {
            attacker,
            rest,
            left,
            right,
        });
        return Some(Pending::ChoosePile {
            player: res.controller,
            piles,
            label: Some(attacker),
        });
    }
    None
}

/// A defending player named their left pile (`chosen`); the rest of their
/// creatures without flying are their right pile.
pub(super) fn resume_divide(
    state: &mut GameState,
    res: &mut Resolution,
    since: u64,
    (asked, rest, mut left, mut right): (PlayerId, Vec<PlayerId>, Vec<ObjectId>, Vec<ObjectId>),
    chosen: &[ObjectId],
) -> Flow {
    for creature in grounded(state, asked) {
        if chosen.contains(&creature) {
            left.push(creature);
        } else {
            right.push(creature);
        }
    }
    match divide_next(state, res, rest, left, right) {
        Some(pending) => next_choice(state, res, since, pending),
        None => finish_choice(state, res, since),
    }
}

/// The controller chose pile `index` (0 left, 1 right) for `attacker`.
pub(super) fn resume_label(
    state: &mut GameState,
    res: &mut Resolution,
    (attacker, rest, left, right): (ObjectId, Vec<ObjectId>, Vec<ObjectId>, Vec<ObjectId>),
    index: usize,
) -> Flow {
    let since = state.journal.last_seq();
    let allowed = if index == 0 {
        left.clone()
    } else {
        right.clone()
    };
    state.combat.limit_blockers_to_pile(attacker, allowed);
    match label_next(state, res, rest, left, right) {
        Some(pending) => next_choice(state, res, since, pending),
        None => finish_choice(state, res, since),
    }
}
