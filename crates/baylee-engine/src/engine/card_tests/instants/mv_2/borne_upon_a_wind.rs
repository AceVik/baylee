//! `cards/instants/mv_2/borne_upon_a_wind.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Borne Upon a Wind ({1}{U}, instant): "You may cast spells this turn as
/// though they had flash. Draw a card." The flash grant is the card's
/// `Coverage::Partial` gap — no modifier hands out a turn-long casting
/// permission — so this plays the half that is written, off two Islands in a
/// first main phase. The draw is asserted on the *card* rather than on a
/// count: the object that was on top of the library is the one that arrives
/// in hand, which is what tells a draw from a spell that merely left the hand.
#[test]
fn borne_upon_a_wind_draws_the_top_card_and_lands_in_the_graveyard() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(83, island())
        .battlefield(0, &[island(), island()])
        .hand(0, &[borne_upon_a_wind()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let library_before = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Library(p0))
        .clone();
    let top = *library_before
        .last()
        .expect("p0 has a library to draw from");
    let hand_before = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))
        .len();
    assert!(
        in_hand(&engine, p0, borne_upon_a_wind()).is_some(),
        "the instant starts in hand"
    );

    cast_from_hand(&mut engine, p0, borne_upon_a_wind());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, borne_upon_a_wind()).is_some(),
        "an instant that has resolved is put into its owner's graveyard"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before.len() - 1,
        "\"Draw a card\" is exactly one off the top, and not two"
    );
    assert!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .contains(&top),
        "the card drawn is the one that was on top of the library"
    );
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .len(),
        hand_before,
        "the spell left the hand and one card replaced it — a draw of zero \
         would leave one fewer"
    );
}
