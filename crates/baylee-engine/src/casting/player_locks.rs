//! Player-scoped casting prohibitions use the actual caster, not card ownership.

use crate::effects::ContinuousEffect;
use crate::object::GameObject;
use crate::state::GameState;
use baylee_cards_dsl::{Filter, PlayerRel};
use baylee_core::ids::PlayerId;

pub(super) fn prevents(
    state: &GameState,
    effect: &ContinuousEffect,
    who: PlayerRel,
    filter: &'static Filter,
    player: PlayerId,
    spell: &GameObject,
) -> bool {
    if !crate::effects::source_is_present(state, effect)
        || effect.source.is_some_and(|source| {
            state
                .object(source)
                .is_some_and(|source| source.status.contains(crate::object::Status::PHASED_OUT))
        })
    {
        return false;
    }
    let affected = match who {
        PlayerRel::ControllerOfAttached => {
            effect
                .source
                .and_then(|source| crate::eval::controller_of_attached(state, source))
                == Some(player)
        }
        PlayerRel::OwnerOfSource => effect
            .source
            .and_then(|source| state.object(source))
            .is_some_and(|source| source.owner == player),
        _ => crate::eval::players(who, state, effect.controller)
            .is_some_and(|players| players.contains(&player)),
    };
    affected
        && crate::eval::matches(
            filter,
            state,
            spell,
            effect.controller,
            effect.source.unwrap_or(spell.id),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::{EffectFilter, EffectOrigin};
    use crate::engine::synthetic::{SyntheticLookup, creature, preset};
    use crate::event::Cause;
    use crate::zone::{ZoneLocation, ZonePosition};
    use baylee_cards_dsl::{Duration, Modifier};
    use baylee_core::ids::EffectId;

    #[test]
    fn attached_controller_cast_lock_uses_actual_caster_and_tracks_host_control() {
        let card = creature(850_003, "Synthetic player lock", 1, 4, &[]);
        let lookup = SyntheticLookup::new(vec![card]);
        let mut state =
            GameState::from_preset(&preset(721, &[850_003, 850_003, 850_003]), &lookup).unwrap();
        let objects = state.zones.list(ZoneLocation::Battlefield).clone();
        let source = objects[0];
        let host = objects[1];
        let spell = objects[2];
        let owner = PlayerId::new(0);
        let borrower = PlayerId::new(1);
        state.object_mut(source).unwrap().attached_to = Some(host);
        state.object_mut(host).unwrap().set_controller(borrower);
        state
            .move_object(
                spell,
                ZoneLocation::Hand(owner),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
        let modifier = Modifier::PlayersCantCast {
            who: PlayerRel::ControllerOfAttached,
            filter: &Filter::CREATURE,
        };
        state.effects.register(ContinuousEffect {
            id: EffectId::new(0),
            source: Some(source),
            controller: owner,
            origin: EffectOrigin::Static,
            layer: modifier.layer(),
            timestamp: 1,
            duration: Duration::WhileSourceOnBattlefield,
            filter: EffectFilter::Dsl(&Filter::Any),
            modifier,
        });
        assert!(!super::super::cast_is_forbidden(
            &state,
            owner,
            state.object(spell).unwrap()
        ));
        assert!(
            super::super::cast_is_forbidden(&state, borrower, state.object(spell).unwrap()),
            "permission to cast somebody else's card still uses the casting player"
        );
        state.object_mut(host).unwrap().set_controller(owner);
        assert!(super::super::cast_is_forbidden(
            &state,
            owner,
            state.object(spell).unwrap()
        ));
        assert!(!super::super::cast_is_forbidden(
            &state,
            borrower,
            state.object(spell).unwrap()
        ));
        let modifier = Modifier::LoseAllAbilities;
        let timestamp = state.next_timestamp();
        state.effects.register(ContinuousEffect {
            id: EffectId::new(0),
            source: None,
            controller: owner,
            origin: EffectOrigin::Resolution,
            layer: modifier.layer(),
            timestamp,
            duration: Duration::UntilEndOfTurn,
            filter: EffectFilter::object(&state, host),
            modifier,
        });
        state.refresh_characteristics();
        assert!(
            super::super::cast_is_forbidden(&state, owner, state.object(spell).unwrap()),
            "removing the host's abilities does not remove the source's casting prohibition"
        );
        state
            .object_mut(source)
            .unwrap()
            .status
            .insert(crate::object::Status::PHASED_OUT);
        assert!(
            !super::super::cast_is_forbidden(&state, owner, state.object(spell).unwrap()),
            "a phased-out source cannot enforce its static prohibition"
        );
    }
}
