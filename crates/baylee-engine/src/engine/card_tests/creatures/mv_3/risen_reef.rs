//! `cards/creatures/mv_3/risen_reef.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Risen Reef: "Whenever this creature … enters, look at the top card of your
/// library. If it's a land card, you may put it onto the battlefield tapped."
#[test]
fn risen_reef_puts_a_land_off_the_top_onto_the_battlefield_tapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), island()])
        .hand(0, &[risen_reef()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let hand = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    cast_from_hand(&mut engine, p0, risen_reef());
    let land = reef_question(&mut engine, p0, true);
    assert_eq!(
        engine.state().object(land).map(|o| o.zone),
        Some(crate::zone::Zone::Battlefield)
    );
    assert!(is_tapped(&engine, land), "onto the battlefield tapped");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand - 1,
        "only the Reef left the hand, and nothing came to it"
    );
}

/// "If you don't put the card onto the battlefield, put it into your hand."
#[test]
fn risen_reef_puts_a_declined_land_into_the_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), island()])
        .hand(0, &[risen_reef()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, risen_reef());
    let land = reef_question(&mut engine, p0, false);
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&land),
        "the declined land went to the hand"
    );
}
