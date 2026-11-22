//! Player-scoped casting prohibitions use the actual caster, not card ownership.

use crate::effects::ContinuousEffect;
use crate::object::GameObject;
use crate::state::GameState;
use baylee_cards_dsl::{Filter, PlayerRel};
use baylee_core::ids::PlayerId;

pub(crate) fn affects_player(
    state: &GameState,
    effect: &ContinuousEffect,
    who: PlayerRel,
    player: PlayerId,
) -> bool {
    if !crate::effects::source_is_present(state, effect)
        || (matches!(
            effect.duration,
            baylee_cards_dsl::Duration::WhileSourceOnBattlefield
        ) && effect.source.is_some_and(|source| {
            state
                .object(source)
                .is_some_and(|source| source.status.contains(crate::object::Status::PHASED_OUT))
        }))
    {
        return false;
    }
    if let Some(seats) = effect.filter.seats() {
        return !state.has_left(player) && seats.contains(player);
    }
    crate::eval::players_for_source(who, state, effect.controller, effect.source)
        .is_some_and(|players| players.contains(&player))
}

pub(super) fn prevents(
    state: &GameState,
    effect: &ContinuousEffect,
    who: PlayerRel,
    filter: &'static Filter,
    player: PlayerId,
    spell: &GameObject,
) -> bool {
    affects_player(state, effect, who, player)
        && crate::eval::matches_with_context(
            filter,
            state,
            spell,
            effect.controller,
            crate::text_changes::RuleContext {
                source: effect.source.unwrap_or(spell.id),
                text: state.effect_text(effect),
                resolved_name: None,
                resolved_subtype: effect.filter.chosen_subtype(),
                reference: None,
            },
        )
}

pub(super) fn matching_limit_reached(
    state: &GameState,
    effect: &ContinuousEffect,
    who: PlayerRel,
    filter: &'static Filter,
    limit: u32,
    player: PlayerId,
    candidate_spell: &GameObject,
) -> bool {
    if !affects_player(state, effect, who, player) {
        return false;
    }
    let src = effect.source.unwrap_or(candidate_spell.id);
    if !crate::eval::matches(filter, state, candidate_spell, effect.controller, src) {
        return false;
    }
    if limit == 0 {
        return true;
    }
    if state.per_turn.spells_cast_by(player) < limit {
        return false;
    }
    let mut moves = std::collections::BTreeMap::<baylee_core::ids::ObjectId, u32>::new();
    let mut cast_matching = 0_u32;
    for entry in state.journal.entries().iter().rev() {
        if matches!(entry.event, crate::event::GameEvent::TurnStarted { .. }) {
            break;
        }
        if let crate::event::GameEvent::ZoneChanged { object, .. } = entry.event {
            *moves.entry(object).or_default() += 1;
            continue;
        }
        let crate::event::GameEvent::SpellCast {
            object,
            player: caster,
            ..
        } = entry.event
        else {
            continue;
        };
        if caster != player {
            continue;
        }
        let Some(current) = state.object_or_departed(object) else {
            continue;
        };
        let reference = baylee_core::ids::DamageSourceRef {
            object,
            version: current
                .version
                .wrapping_sub(moves.get(&object).copied().unwrap_or(0)),
        };
        let spell = state.source_object(reference).unwrap_or(current);
        if crate::eval::matches(filter, state, spell, effect.controller, src) {
            cast_matching = cast_matching.saturating_add(1);
            if cast_matching >= limit {
                return true;
            }
        }
    }
    false
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
