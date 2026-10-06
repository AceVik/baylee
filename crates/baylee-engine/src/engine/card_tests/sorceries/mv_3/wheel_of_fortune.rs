//! `cards/sorceries/mv_3/wheel_of_fortune.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wheel of Fortune: "Each player discards their hand, then draws seven cards."
/// Three Mountains pay {2}{R} to cast the sorcery; the card left in each
/// hand is discarded, and both players draw seven cards from their
/// libraries — so seven is what each hand holds, and not eight.
#[test]
fn wheel_of_fortune_each_player_discards_their_hand_and_draws_seven_cards() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(39, forest())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(0, &[wheel_of_fortune(), lightning_bolt()])
        .hand(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    let wheel = in_hand(&engine, p0, wheel_of_fortune()).expect("Wheel of Fortune in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: wheel })
        .expect("three Mountains pay {2}{R}");

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        1,
        "the Bolt is still in hand as the Wheel goes on the stack"
    );
    let p0_lib_before = library_size(&engine, p0);
    let p1_lib_before = library_size(&engine, p1);

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, lightning_bolt()).is_some(),
        "p0 discarded the Bolt"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "p1 discarded the Elves"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        7,
        "p0 holds the seven cards drawn and nothing else"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        7,
        "p1 holds the seven cards drawn and nothing else"
    );
    assert_eq!(
        library_size(&engine, p0),
        p0_lib_before - 7,
        "p0 library reduced by seven"
    );
    assert_eq!(
        library_size(&engine, p1),
        p1_lib_before - 7,
        "p1 library reduced by seven"
    );
    assert!(
        in_graveyard(&engine, p0, wheel_of_fortune()).is_some(),
        "Wheel of Fortune went to graveyard upon resolution"
    );
}
