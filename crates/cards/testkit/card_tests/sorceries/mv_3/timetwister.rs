//! `cards/sorceries/mv_3/timetwister.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Timetwister: "Each player shuffles their hand and graveyard into their
/// library, then draws seven cards."
#[test]
fn timetwister_shuffles_hand_and_graveyard_and_draws_seven_for_each_player() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[timetwister(), lightning_bolt()])
        .hand(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    hand_to_graveyard(&mut engine, p0, lightning_bolt());
    hand_to_graveyard(&mut engine, p1, quiet_creature());

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, timetwister());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        7,
        "\"draws seven cards\""
    );
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(p1)).len(), 7);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        1,
        "only Timetwister itself remains — its own graveyard and hand were shuffled away first"
    );
    assert!(in_graveyard(&engine, p0, timetwister()).is_some());
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p1))
            .is_empty(),
        "p1's graveyard was shuffled in too"
    );
}
