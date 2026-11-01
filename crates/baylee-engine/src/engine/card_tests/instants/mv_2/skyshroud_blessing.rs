//! `cards/instants/mv_2/skyshroud_blessing.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

/// Skyshroud Blessing — {1}{G} Instant
/// "All lands gain shroud until end of turn.
/// Draw a card."
#[test]
fn skyshroud_blessing_grants_all_lands_shroud_and_draws_a_card() {
    let blessing = card_index("ffcedb73-538a-45ea-8564-7b92624537e0");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .battlefield(1, &[island()])
        .hand(0, &[blessing])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let p0_land = on_battlefield(&engine, p0, forest()).unwrap();
    let p1_land = on_battlefield(&engine, p1, island()).unwrap();
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, blessing);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );
    assert!(
        engine
            .state()
            .object(p0_land)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::SHROUD)
    );
    assert!(
        engine
            .state()
            .object(p1_land)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::SHROUD)
    );
}
