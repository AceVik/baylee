//! An exact permanent can replace every non-exile battlefield departure.

use crate::state::GameState;
use crate::zone::{Zone, ZoneLocation};
use baylee_core::ids::ObjectId;

pub(crate) fn destination(state: &GameState, id: ObjectId, to: ZoneLocation) -> ZoneLocation {
    if matches!(to, ZoneLocation::Battlefield | ZoneLocation::Exile(_)) {
        return to;
    }
    let Some(object) = state
        .object(id)
        .filter(|object| object.zone == Zone::Battlefield)
    else {
        return to;
    };
    if state.effects.iter().any(|effect| {
        effect.modifier == baylee_cards_dsl::Modifier::ExileInsteadOfLeavingBattlefield
            && crate::effects::applies_to(state, effect, object)
    }) {
        ZoneLocation::Exile(object.owner)
    } else {
        to
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::{ContinuousEffect, EffectFilter, EffectOrigin};
    use crate::engine::synthetic::{SyntheticLookup, preset};
    use crate::event::Cause;
    use crate::zone::ZonePosition;
    use baylee_cards_dsl::{Duration, Modifier};
    use baylee_core::ids::{EffectId, PlayerId};

    #[test]
    fn replacement_exiles_each_nonexile_exit_but_never_a_later_incarnation() {
        let player = PlayerId::new(0);
        for requested in [
            ZoneLocation::Hand(player),
            ZoneLocation::Graveyard(player),
            ZoneLocation::Library(player),
            ZoneLocation::Exile(player),
        ] {
            let lookup = SyntheticLookup::new(vec![]);
            let mut state = GameState::from_preset(&preset(716, &[]), &lookup).unwrap();
            let object = state.zones.list(ZoneLocation::Library(player))[0];
            state.object_mut(object).unwrap().kind = crate::object::ObjectKind::Permanent;
            state
                .move_object(
                    object,
                    ZoneLocation::Battlefield,
                    ZonePosition::Top,
                    Cause::Effect,
                )
                .unwrap();
            let modifier = Modifier::ExileInsteadOfLeavingBattlefield;
            state.effects.register(ContinuousEffect {
                id: EffectId::new(0),
                source: None,
                controller: player,
                origin: EffectOrigin::Resolution,
                layer: modifier.layer(),
                timestamp: 1,
                duration: Duration::Indefinitely,
                filter: EffectFilter::object(&state, object),
                modifier,
            });
            assert_eq!(
                destination(&state, object, requested),
                ZoneLocation::Exile(player)
            );
            state
                .move_object(object, requested, ZonePosition::Top, Cause::Effect)
                .unwrap();
            assert_eq!(state.object(object).unwrap().zone, Zone::Exile);
            state.object_mut(object).unwrap().kind = crate::object::ObjectKind::Permanent;
            state
                .move_object(
                    object,
                    ZoneLocation::Battlefield,
                    ZonePosition::Top,
                    Cause::Effect,
                )
                .unwrap();
            assert_eq!(
                destination(&state, object, ZoneLocation::Hand(player)),
                ZoneLocation::Hand(player)
            );
            state
                .move_object(
                    object,
                    ZoneLocation::Hand(player),
                    ZonePosition::Top,
                    Cause::Effect,
                )
                .unwrap();
            assert_eq!(state.object(object).unwrap().zone, Zone::Hand);
        }
    }
}
