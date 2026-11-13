use super::{hits, me, permanent, state, them};
use crate::effects::{ContinuousEffect, EffectFilter, EffectOrigin};
use crate::event::{Cause, GameEvent};
use crate::object::ObjectKind;
use crate::state::GameState;
use crate::zone::{ZoneLocation, ZonePosition};
use baylee_cards_dsl::{Duration, Filter, Modifier, Trigger};
use baylee_core::ids::EffectId;

#[test]
fn ward_suppression_matches_the_targeted_permanent_while_the_source_is_present() {
    let mut state = state();
    let source = permanent(&mut state, me(), "Ward suppression");
    let target = permanent(&mut state, them(), "Ward recipient");
    let name = state.names.intern("Targeting spell");
    let spell = state.create_bare(me(), ObjectKind::Spell, name, ZoneLocation::Stack);
    let event = GameEvent::BecameTarget {
        object: spell,
        target,
        controller: me(),
    };
    let ward_hits = |state: &GameState| hits(&Trigger::Ward, &event, &[], state, target, them());
    assert_eq!(ward_hits(&state), 1);

    let modifier = Modifier::WardDoesntTrigger;
    for (filter, expected) in [
        (&Filter::ControlledByYou, 1),
        (&Filter::ControlledByOpponent, 0),
    ] {
        state.effects.register(ContinuousEffect {
            id: EffectId::new(0),
            source: Some(source),
            controller: me(),
            origin: EffectOrigin::Resolution,
            layer: modifier.layer(),
            timestamp: 1,
            duration: Duration::WhileSourceOnBattlefield,
            filter: EffectFilter::Dsl(filter),
            modifier,
        });
        assert_eq!(ward_hits(&state), expected, "filter {filter:?}");
    }

    state
        .move_object(
            source,
            ZoneLocation::Graveyard(me()),
            ZonePosition::Top,
            Cause::Effect,
        )
        .expect("the suppression source leaves the battlefield");
    assert_eq!(ward_hits(&state), 1);
}
