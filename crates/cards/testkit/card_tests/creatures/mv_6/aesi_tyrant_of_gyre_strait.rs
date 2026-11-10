//! `cards/creatures/mv_6/aesi_tyrant_of_gyre_strait.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Aesi, Tyrant of Gyre Strait: the extra land drop and the landfall draw,
/// which only a turn that plays two lands can tell apart.
///
/// The second land is the one that proves `ExtraLandDrops(1)` — CR 305.2
/// allows one a turn — and each of the two asks the "you may draw a card"
/// question, so the hand is down two lands and up two draws.
#[test]
fn aesi_plays_a_second_land_and_draws_off_each_one() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(385, forest())
        .battlefield(0, &[aesi_tyrant_of_gyre_strait()])
        .hand(0, &[forest(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let before = library_size(&engine, p0);
    for land in [forest(), island()] {
        let card = in_hand(&engine, p0, land).expect("the land is in hand");
        engine
            .apply(p0, PlayerAction::PlayLand { card })
            .expect("the land drop is allowed");
        pass_until(&mut engine, stack_is_empty);
    }

    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .filter(|id| engine
                .state()
                .object(**id)
                .is_some_and(|o| o.characteristics().types.intersects(TypeSet::LAND)))
            .count(),
        2,
        "both lands are on the battlefield, so the second drop was allowed"
    );
    assert_eq!(
        library_size(&engine, p0),
        before - 2,
        "and landfall drew a card for each of them"
    );
}
