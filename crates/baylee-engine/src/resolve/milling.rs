//! Milling batches and the inherent radiation instruction.
use super::{
    Amount, Cause, GameState, ObjectId, Pending, PlayerId, PlayerRel, Resolution, amount2,
    players_of,
};
use baylee_cards_dsl::CounterKind;
use baylee_core::types::TypeSet;

pub(super) fn mill(
    state: &mut GameState,
    res: &mut Resolution,
    amount: Amount,
    target: PlayerRel,
) -> Option<Pending> {
    let n = amount2(&amount, state, res.controller, res) as usize;
    for player in players_of(target, state, res.controller, res) {
        let _ = mill_player(state, player, n);
    }
    None
}

/// The full set is frozen before any move; replacements use the ordinary zone
/// door, and the caller can find moved cards in public destinations (CR 701.17c).
fn mill_player(state: &mut GameState, player: PlayerId, count: usize) -> Vec<ObjectId> {
    state.mill_top(player, count)
}

pub(super) fn radiation(state: &mut GameState, res: &Resolution) {
    // EventPlayer survives copying and control changes of the ability: "that
    // player" refers to the active player who caused it to trigger, not to a
    // later controller of the stack object (CR 728.1, 707.10).
    for player in players_of(PlayerRel::EventPlayer, state, res.controller, res) {
        let count = state.players[usize::from(player.get())].rad;
        // The intervening condition is checked again at resolution (CR 603.4).
        if count == 0 {
            continue;
        }
        let cards = mill_player(state, player, usize::from(count));
        let nonlands = cards
            .into_iter()
            .filter(|id| {
                state.object(*id).is_some_and(|o| {
                    !o.zone.is_hidden_by_default()
                        && !o.status.contains(crate::object::Status::FACE_DOWN)
                        && !o.characteristics().types.contains(TypeSet::LAND)
                })
            })
            .count();
        let loss = i32::try_from(nonlands).expect("mill count bounded by held rad counters");
        if loss > 0 {
            state.change_life(player, -loss, Cause::Radiation);
            state.change_player_counters(player, CounterKind::Rad, -loss);
        }
    }
}
