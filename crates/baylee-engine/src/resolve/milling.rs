//! Milling a batch of cards from the chosen libraries.
use super::{
    Amount, Cause, GameState, ObjectId, Pending, PlayerRel, Resolution, ZoneLocation, ZonePosition,
    amount2,
};

pub(super) fn mill(
    state: &mut GameState,
    res: &mut Resolution,
    amount: Amount,
    target: PlayerRel,
) -> Option<Pending> {
    let you = res.controller;
    let n = amount2(&amount, state, you, res) as usize;
    for player in super::players_of(target, state, you, res) {
        let top: Vec<ObjectId> = state
            .zones
            .list(ZoneLocation::Library(player))
            .iter()
            .rev()
            .take(n)
            .copied()
            .collect();
        for card in top {
            let _ = state.move_object(
                card,
                ZoneLocation::Graveyard(player),
                ZonePosition::Top,
                Cause::Effect,
            );
        }
    }
    None
}
