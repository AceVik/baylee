//! Glyph of Reincarnation's second sentence (`Effect::ReturnPerBlockedDeath`):
//! "For each creature that died this way, put a creature card from the
//! graveyard of the player who controlled that creature the last time it
//! became blocked by that Wall onto the battlefield under its owner's
//! control."
//!
//! Every death is in the graveyard first; then, graveyard by graveyard in
//! turn order from the active player, the controller picks as many cards as
//! that player owes, and they all enter together (the 2004 ruling: "First
//! place all the affected creatures in the graveyard, then choose …").

use baylee_cards_dsl::Filter;
use baylee_core::ids::{ObjectId, PlayerId};

use super::{AwaitingOp, Resolution};
use crate::choice::{ChoicePrompt, Pending};
use crate::eval;
use crate::event::GameEvent;
use crate::state::GameState;
use crate::zone::{Zone, ZoneLocation};

/// The graveyards still to pick from, and what was picked.
#[derive(Clone, Debug, Hash)]
pub struct Returns {
    filter: &'static Filter,
    owed: Vec<(PlayerId, u32)>,
    picked: Vec<ObjectId>,
}

pub(super) fn start(
    state: &mut GameState,
    res: &mut Resolution,
    filter: &'static Filter,
) -> Option<Pending> {
    let Some(wall) = res.rule_context().reference else {
        return None;
    };
    let mut owed: Vec<(PlayerId, u32)> = Vec::new();
    for entry in super::this_way::entries(state, res) {
        let GameEvent::ZoneChanged {
            object,
            from: Zone::Battlefield,
            to: Zone::Graveyard,
            ..
        } = entry.event
        else {
            continue;
        };
        let Some(record) = state
            .per_turn
            .blocks
            .iter()
            .rev()
            .find(|r| r.blocker.object == wall && r.attacker.object == object)
        else {
            continue;
        };
        match owed.iter_mut().find(|(p, _)| *p == record.attacker_controller) {
            Some((_, n)) => *n += 1,
            None => owed.push((record.attacker_controller, 1)),
        }
    }
    let seats = state.players.len();
    let active = usize::from(state.turn.active.get());
    owed.sort_by_key(|(p, _)| (usize::from(p.get()) + seats - active) % seats);
    ask(
        state,
        res,
        Returns {
            filter,
            owed,
            picked: Vec::new(),
        },
    )
}

pub(super) fn resume(
    state: &mut GameState,
    res: &mut Resolution,
    mut returns: Returns,
    chosen: &[ObjectId],
) -> Option<Pending> {
    returns.picked.extend_from_slice(chosen);
    returns.owed.remove(0);
    ask(state, res, returns)
}

fn ask(state: &mut GameState, res: &mut Resolution, mut returns: Returns) -> Option<Pending> {
    let you = res.controller;
    while let Some(&(player, n)) = returns.owed.first() {
        let options: Vec<ObjectId> = state
            .zones
            .list(ZoneLocation::Graveyard(player))
            .iter()
            .copied()
            .filter(|id| !returns.picked.contains(id))
            .filter(|id| {
                state.object(*id).is_some_and(|o| {
                    eval::matches_with_context(returns.filter, state, o, you, res.rule_context())
                })
            })
            .collect();
        let k = u8::try_from(u32::try_from(options.len()).unwrap_or(u32::MAX).min(n))
            .unwrap_or(u8::MAX);
        if k == 0 {
            returns.owed.remove(0);
            continue;
        }
        res.awaiting = Some(AwaitingOp::BlockedReturns(Box::new(returns)));
        return Some(Pending::ChooseCards {
            player: you,
            options,
            min: k,
            max: k,
            prompt: ChoicePrompt::FromGraveyard,
            total: None,
        });
    }
    for card in returns.picked {
        super::reanimation::return_card(state, card, you, true, None);
    }
    None
}
