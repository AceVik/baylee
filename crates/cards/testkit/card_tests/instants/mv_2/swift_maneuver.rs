//! `cards/instants/mv_2/swift_maneuver.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;
use baylee_engine::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "d7b5fb4b-8f3c-4143-9fb0-2e2544612528"
fn swift_maneuver() -> CardIndex {
    card_index("d7b5fb4b-8f3c-4143-9fb0-2e2544612528")
}

/// Swift Maneuver ({1}{W}): prevents the next 2 damage to any target (tested on
/// both a creature and a player) this turn, does not draw immediately, and
/// draws 1 card at the beginning of the next turn's upkeep.
#[test]
fn swift_maneuver_prevents_next_two_damage_to_any_target_and_draws_next_upkeep() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[],
        &[swift_maneuver(), swift_maneuver()],
        &[index::LIGHTNING_BOLT, index::LIGHTNING_BOLT],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);

    // Shield Bears (2/2) and P0 against 2 damage each.
    lk::float(&mut engine, P0, &[(ManaColor::White, 4)]);
    cast_with_floating(&mut engine, P0, swift_maneuver());
    lk::aim(&mut engine, &[bears], &[]);
    lk::settle(&mut engine);

    cast_with_floating(&mut engine, P0, swift_maneuver());
    lk::aim(&mut engine, &[], &[P0]);
    lk::settle(&mut engine);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        0,
        "does not draw immediately"
    );

    // P1 casts Lightning Bolt at Bears (3 - 2 = 1 -> survives) and at P0 (3 - 2 = 1 -> 19).
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    lk::float(&mut engine, P1, &[(ManaColor::Red, 2)]);
    cast_with_floating(&mut engine, P1, index::LIGHTNING_BOLT);
    lk::aim(&mut engine, &[bears], &[]);
    lk::settle(&mut engine);
    assert_eq!(engine.state().object(bears).unwrap().damage, 1);

    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    cast_with_floating(&mut engine, P1, index::LIGHTNING_BOLT);
    lk::aim(&mut engine, &[], &[P0]);
    lk::settle(&mut engine);
    assert_eq!(lk::life(&engine, P0), 19, "2 of 3 damage prevented");

    // Next turn's upkeep: both delayed triggers draw 1 card each.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && e.state().turn.step == Step::Upkeep
    });
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        2,
        "drew 2 cards on next turn's upkeep"
    );
}
