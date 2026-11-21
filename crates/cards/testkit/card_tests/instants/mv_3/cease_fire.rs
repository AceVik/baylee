//! `cards/instants/mv_3/cease_fire.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "9e48eb8e-ec62-4d73-ac80-31e6fe74ec31"
fn cease_fire() -> CardIndex {
    card_index("9e48eb8e-ec62-4d73-ac80-31e6fe74ec31")
}

/// Cease-Fire ({2}{W}): stops target player from casting creature spells this
/// turn (while still allowing noncreature spells), draws a card, and wears off
/// on the next turn.
#[test]
fn cease_fire_forbids_only_creature_spells_this_turn_and_draws() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[cease_fire()])
        .battlefield(1, &[forest(), forest(), mountain()])
        .hand(1, &[index::GRIZZLY_BEARS, lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);

    let hand0_before = engine.state().zones.list(ZoneLocation::Hand(P0)).len();

    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, cease_fire());
    engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand0_before,
        "cast 1 spell, drew 1 card -> net 0"
    );

    // P1 cannot cast Grizzly Bears this turn, but CAN cast noncreature Bolt.
    tap_all_mana(&mut engine, P1);
    assert!(
        !lk::castable(&engine, P1, index::GRIZZLY_BEARS),
        "creature spells are forbidden this turn"
    );
    assert!(
        lk::castable(&engine, P1, lightning_bolt()),
        "noncreature spells are still castable"
    );
    lk::cast_at(&mut engine, P1, lightning_bolt(), &[], &[P0]);
    assert_eq!(lk::life(&engine, P0), 17);

    // Next turn P1 can cast creature spells again.
    reach_their_main_phase(&mut engine, P0);
    reach_their_main_phase(&mut engine, P1);
    tap_all_mana(&mut engine, P1);
    assert!(lk::castable(&engine, P1, index::GRIZZLY_BEARS));
}
