//! Resolution effects that prohibit regeneration for the rest of a turn.

use super::{Resolution, zones};
use crate::state::GameState;
use baylee_cards_dsl::TargetSpec;

pub(super) fn prevent_for_targets(state: &mut GameState, res: &Resolution, target: TargetSpec) {
    for id in zones::spec_objects(state, res, target) {
        if let Some(obj) = state.object(id)
            && obj.zone == crate::zone::Zone::Battlefield
        {
            let named = (id, obj.version);
            if !state.per_turn.cant_regenerate.contains(&named) {
                state.per_turn.cant_regenerate.push(named);
            }
        }
    }
}
