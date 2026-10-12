//! Control expiry, ability removal and multiplayer departure at turn boundaries.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::effects::{ContinuousEffect, EffectFilter, EffectOrigin};
use baylee_cards_dsl::{Duration, Modifier};
use baylee_core::ids::EffectId;

fn exact_effect(
    e: &mut Engine<RegistryLookup>,
    bear: ObjectId,
    controller: PlayerId,
    modifier: Modifier,
    duration: Duration,
) {
    let version = e.state.object(bear).unwrap().version;
    let timestamp = e.state.next_timestamp();
    e.state.effects.register(ContinuousEffect {
        id: EffectId::new(0),
        source: None,
        controller,
        origin: EffectOrigin::Resolution,
        layer: modifier.layer(),
        timestamp,
        duration,
        filter: EffectFilter::ObjectIs(bear, version),
        modifier,
    });
    e.state.invalidate_projections();
    e.sync_static_effects();
    e.refresh_offer();
}

#[test]
fn wall_waiting_rule_survives_its_effect_controllers_multiplayer_concession() {
    let mut e = Duel::table(SEED, index::FOREST, 3)
        .battlefield(0, &[index::WALL_OF_DUST])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let (wall, bear) = block(&mut e);
    pass_until(&mut e, stack_is_empty);
    e.apply(P0, PlayerAction::Concede).unwrap();
    assert!(e.state.object(wall).is_none());
    assert_eq!(e.state.object(bear).unwrap().controller, P1);
    next_main(&mut e, P1);
    attack_offer(&mut e, P1, bear, false);
    next_main(&mut e, P1);
    attack_offer(&mut e, P1, bear, true);
}

#[test]
fn wall_waiting_rule_uses_controller_after_until_next_turn_control_expires() {
    let mut e = prepared(&[], &[index::WALL_OF_DUST], &[index::GRIZZLY_BEARS]);
    let (_, bear) = block(&mut e);
    pass_until(&mut e, stack_is_empty);
    // No Implemented helper creates this precise control-duration boundary.
    // The real Wall trigger is already resolved; only the unrelated control
    // effect is installed as a transparent continuous-effect fixture.
    exact_effect(
        &mut e,
        bear,
        P0,
        Modifier::GainControl,
        Duration::UntilYourNextTurn,
    );
    assert_eq!(e.state.object(bear).unwrap().controller, P0);
    next_main(&mut e, P0);
    assert_eq!(e.state.object(bear).unwrap().controller, P1);
    // The expired P0 control must not consume the waiting rule at that P0
    // boundary: the rule starts during the next actual turn of restored P1.
    next_main(&mut e, P1);
    attack_offer(&mut e, P1, bear, false);
    next_main(&mut e, P1);
    attack_offer(&mut e, P1, bear, true);
}

#[test]
fn wall_attack_restriction_is_a_rule_that_losing_all_abilities_does_not_remove() {
    let mut e = prepared(&[], &[index::WALL_OF_DUST], &[index::GRIZZLY_BEARS]);
    let (_, bear) = block(&mut e);
    pass_until(&mut e, stack_is_empty);
    next_main(&mut e, P1);
    // Timestamp after the Wall restriction is active. This would remove a
    // CANT_ATTACK keyword granted as an ability, but cannot remove the rule.
    exact_effect(
        &mut e,
        bear,
        P1,
        Modifier::LoseAllAbilities,
        Duration::UntilEndOfTurn,
    );
    assert!(
        !e.state
            .object(bear)
            .unwrap()
            .characteristics()
            .keywords
            .contains(baylee_cards_dsl::KeywordSet::CANT_ATTACK)
    );
    attack_offer(&mut e, P1, bear, false);
    next_main(&mut e, P1);
    attack_offer(&mut e, P1, bear, true);
}
