//! `cards/lands/bounce/azorius_chancery.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The bounce land alone on the battlefield returns **itself**.
///
/// One row is enough because the rule is the same on all eleven, and this is
/// the board that says the menu is not filtered down to "some other land":
/// with nothing else in play the only legal answer is the source, the player
/// is still asked, and the land goes back to the hand it was just played
/// from. An engine that excluded the source would have to ask an empty
/// question here, which is the dead end `min: 1` forbids.
#[test]
fn a_bounce_land_alone_on_the_battlefield_returns_itself() {
    let p0 = PlayerId::new(0);
    let chancery = card_index("189fc8f4-17ac-4f1d-82c8-8401445bdaf4");
    let mut engine = Duel::new(971, forest()).hand(0, &[chancery]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, chancery);
    let (options, _) =
        reach_the_unless_question(&mut engine, land).expect("the land asks even with one answer");
    assert_eq!(options, vec![land], "the source is the whole menu");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![land],
            },
        )
        .unwrap();

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&land),
        "it returned itself to its owner's hand"
    );
    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&land),
        "and is no longer on the battlefield"
    );
}
