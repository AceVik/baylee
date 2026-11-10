//! `cards/sorceries/mv_6/imperial_oath.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "4a4a061d-bc4e-49da-8d20-90fcb571b025"
fn imperial_oath() -> CardIndex {
    card_index("4a4a061d-bc4e-49da-8d20-90fcb571b025")
}

/// Imperial Oath: "Create three 2/2 white Samurai creature tokens with
/// vigilance. Scry 3."
///
/// The scry looks at three cards, and sending one to the bottom is read off the
/// library.
#[test]
fn imperial_oath_makes_three_vigilant_samurai_and_scries_three() {
    let mut engine = table(962, &[imperial_oath()], &[plains(); 6], &[], &[]);
    cast_from_hand(&mut engine, me(), imperial_oath());
    let cards = look_and_send(&mut engine, 1);
    assert_eq!(cards.len(), 3, "scry 3 looks at three");
    settle(&mut engine);

    let samurai = my_tokens(&engine);
    assert_eq!(samurai.len(), 3);
    for token in samurai {
        assert_eq!(pt(&engine, token), (2, 2));
        assert!(
            keywords_of(&engine, token).contains(KeywordSet::VIGILANCE),
            "vigilance"
        );
    }
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(me()))
            .first(),
        Some(&cards[0]),
        "the card sent away is now the bottom of the library"
    );
}
