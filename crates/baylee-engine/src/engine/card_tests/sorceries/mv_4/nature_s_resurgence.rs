//! `cards/sorceries/mv_4/nature_s_resurgence.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRAY_OGRE, GRIZZLY_BEARS, HILL_GIANT, NATURE_S_RESURGENCE};

/// `Nature's Resurgence` ({2}{G}{G}, Sorcery, Partial) draws a card for each
/// player per creature card in their graveyard.
#[test]
fn nature_s_resurgence_draws_cards_for_creatures_in_graveyards() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[NATURE_S_RESURGENCE, GRIZZLY_BEARS, HILL_GIANT])
        .hand(1, &[GRAY_OGRE])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    hand_to_graveyard(&mut engine, p0, GRIZZLY_BEARS);
    hand_to_graveyard(&mut engine, p0, HILL_GIANT);
    hand_to_graveyard(&mut engine, p1, GRAY_OGRE);
    let hand0_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let hand1_before = engine.state().zones.list(ZoneLocation::Hand(p1)).len();

    cast_from_hand(&mut engine, p0, NATURE_S_RESURGENCE);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand0_before - 1 + 2
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        hand1_before + 1
    );
}
