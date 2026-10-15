//! Starting speed is a state-based action; the designation outlives its source.

use crate::state::{CardLookup, GameState};

pub(super) fn start(state: &mut GameState, lookup: &impl CardLookup) -> bool {
    if state.players.iter().all(|p| p.has_lost() || p.speed != 0) {
        return false;
    }
    let mut started = Vec::new();
    for id in state.battlefield_seen() {
        let Some(object) = state.object(id) else {
            continue;
        };
        let player = &state.players[usize::from(object.controller.get())];
        if player.speed == 0
            && !player.has_lost()
            && object
                .abilities(lookup)
                .iter()
                .any(|a| matches!(a, baylee_cards_dsl::AbilityDef::StartYourEngines))
        {
            started.push(object.controller);
        }
    }
    for player in &started {
        state.players[usize::from(player.get())].speed = 1;
    }
    !started.is_empty()
}
