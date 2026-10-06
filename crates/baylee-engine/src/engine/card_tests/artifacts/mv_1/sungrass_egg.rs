//! `cards/artifacts/mv_1/sungrass_egg.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Sungrass Egg` prints `{{2}}, {{T}}, Sacrifice this artifact: Add {{G}}{{W}}. Draw a card.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Sungrass Egg` and two copies of `forest()`.
/// Two floating mana pay the ordinary activated ability. The Egg is sacrificed
/// immediately; one green, one white and the drawn card arrive on resolution.
#[test]
fn sungrass_egg_filters_mana_and_draws_a_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), sungrass_egg()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let initial_hand = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    tap_all_mana_but(&mut engine, p0, Some(sungrass_egg()));
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);

    activate(&mut engine, p0, sungrass_egg(), 0);
    assert_eq!(engine.state().zones.list(ZoneLocation::Stack).len(), 1);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        initial_hand
    );
    assert!(in_graveyard(&engine, p0, sungrass_egg()).is_some());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        stack_is_empty(&engine),
        "the activated ability has finished resolving"
    );
    assert!(
        in_graveyard(&engine, p0, sungrass_egg()).is_some(),
        "`Sungrass Egg` was sacrificed"
    );

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1, "added green mana");
    assert_eq!(pool.available(ManaColor::White), 1, "added white mana");
    assert_eq!(pool.total(), 2, "exactly two mana in pool");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        initial_hand + 1,
        "drew a card"
    );
}
