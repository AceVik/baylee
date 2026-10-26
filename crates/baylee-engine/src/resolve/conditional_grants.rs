//! Grant a triggered ability only while the subject does not already have it.

use super::{Resolution, subjects, zones};
use crate::effects::{ContinuousEffect, EffectFilter, EffectOrigin};
use crate::state::GameState;
use baylee_cards_dsl::{Duration, Effect, Modifier, TargetSpec, Trigger};
use baylee_core::ids::EffectId;

pub(super) fn grant(
    state: &mut GameState,
    res: &Resolution,
    target: TargetSpec,
    trigger: Trigger,
    effects: &'static [Effect],
    duration: Duration,
) {
    let modifier = Modifier::GrantTriggered {
        trigger,
        effects,
        target: None,
    };
    let expected = baylee_cards_dsl::triggered!(trigger, effects);
    for id in zones::spec_objects(state, res, target) {
        let reference = match target {
            TargetSpec::ThisObject => subjects::source(state, res),
            TargetSpec::EventObject => subjects::event(state, res),
            _ => subjects::target(state, res, id),
        };
        let Some(reference) =
            reference.filter(|reference| subjects::on_battlefield(state, *reference))
        else {
            continue;
        };
        let object = state.object(id).expect("current battlefield subject");
        let printed = object.characteristics().abilities_lost.is_none()
            && !object.characteristics().rules_text_lost
            && state
                .printed_ability_list(id)
                .is_some_and(|list| list.abilities.iter().any(|ability| ability == &expected));
        let granted = state.effects.iter().any(|effect| {
            effect.modifier == modifier && crate::effects::applies_to(state, effect, object)
        });
        if printed || granted {
            continue;
        }
        let filter = EffectFilter::ObjectIs(id, reference.version);
        if !crate::permanent_guards::permits_resolution_effect(
            state,
            filter,
            modifier,
            res.controller,
        ) {
            continue;
        }
        let timestamp = state.next_timestamp();
        state.effects.register(ContinuousEffect {
            id: EffectId::new(0),
            source: Some(res.source),
            controller: res.controller,
            origin: EffectOrigin::Resolution,
            layer: modifier.layer(),
            timestamp,
            duration,
            filter,
            modifier,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::synthetic::{SyntheticLookup, creature, preset};
    use crate::event::Cause;
    use crate::resolve::SubjectContext;
    use crate::zone::{ZoneLocation, ZonePosition};
    use baylee_cards_dsl::{CounterKind, Filter, PlayerRel, StepKind};
    use baylee_core::ids::{PlayerId, SeatSet};
    use smallvec::SmallVec;

    #[test]
    #[allow(clippy::too_many_lines)] // One scenario follows grant identity through removal and departure.
    fn conditional_grant_checks_actual_ability_not_counters_and_survives_source_departure() {
        static EFFECTS: &[Effect] = &[Effect::SacrificeSelf];
        let card = creature(850_004, "Synthetic conditional grant", 1, 4, &[]);
        let lookup = SyntheticLookup::new(vec![card]);
        let mut state = GameState::from_preset(&preset(722, &[850_004, 850_004]), &lookup).unwrap();
        let objects = state.zones.list(ZoneLocation::Battlefield).clone();
        let source = objects[0];
        let target = objects[1];
        let player = PlayerId::new(0);
        let res = Resolution {
            source,
            on_stack: source,
            controller: player,
            effects: vec![],
            pc: 0,
            targets: smallvec::smallvec![target],
            second_targets: SmallVec::new(),
            x: None,
            chosen_player: None,
            target_lki: None,
            subject: SubjectContext::default(),
            text: crate::text_changes::TextChangeMap::IDENTITY,
            event_mana: None,
            retarget_left: None,
            loot: None,
            draw_watch: None,
            target_players: SeatSet::new(),
            event_object: None,
            awaiting: None,
            targeted: true,
            mana_ability: false,
            countered_source: None,
            this_way_from: None,
            coin_stake: None,
        };
        let trigger = Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        };
        let modifier = Modifier::GrantTriggered {
            trigger,
            effects: EFFECTS,
            target: None,
        };
        let active = |state: &GameState| {
            state
                .effects
                .iter()
                .filter(|effect| {
                    effect.modifier == modifier
                        && crate::effects::applies_to(state, effect, state.object(target).unwrap())
                })
                .count()
        };
        crate::replacement::put_counters(&mut state, target, CounterKind::P1P1, 2);
        grant(
            &mut state,
            &res,
            TargetSpec::Object(&Filter::Any),
            trigger,
            EFFECTS,
            Duration::Indefinitely,
        );
        assert_eq!(
            active(&state),
            1,
            "existing counters do not substitute for an ability"
        );
        grant(
            &mut state,
            &res,
            TargetSpec::Object(&Filter::Any),
            trigger,
            EFFECTS,
            Duration::Indefinitely,
        );
        assert_eq!(
            active(&state),
            1,
            "an identical existing trigger is not granted again"
        );
        let lost = Modifier::LoseAllAbilities;
        let timestamp = state.next_timestamp();
        state.effects.register(ContinuousEffect {
            id: EffectId::new(0),
            source: None,
            controller: player,
            origin: EffectOrigin::Resolution,
            layer: lost.layer(),
            timestamp,
            duration: Duration::Indefinitely,
            filter: EffectFilter::object(&state, target),
            modifier: lost,
        });
        state.refresh_characteristics();
        assert_eq!(active(&state), 0);
        grant(
            &mut state,
            &res,
            TargetSpec::Object(&Filter::Any),
            trigger,
            EFFECTS,
            Duration::Indefinitely,
        );
        assert_eq!(
            active(&state),
            1,
            "an ability removed by a later effect can be granted anew"
        );
        state
            .move_object(
                source,
                ZoneLocation::Graveyard(player),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
        assert_eq!(
            active(&state),
            1,
            "the grant is independent of its original source"
        );
    }
}
