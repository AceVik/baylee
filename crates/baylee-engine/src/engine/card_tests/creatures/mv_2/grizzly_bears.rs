//! `cards/creatures/mv_2/grizzly_bears.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The play test for stubgen's vanilla rule: a card that prints no rules
/// text is written with nothing but its face, and that face is the card.
/// Grizzly Bears is cast for the `{1}{G}` it prints and stands on the
/// battlefield as the 2/2 it prints, with no ability to offer.
#[test]
fn grizzly_bears_is_cast_for_its_cost_and_is_the_two_two_it_prints() {
    let p0 = PlayerId::new(0);
    let bears = card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0");
    let mut engine = Duel::new(7, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[bears])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, bears);
    assert!(matches!(drive_to_rest(&mut engine, p0), Rest::Reached));
    let object = on_battlefield(&engine, p0, bears).expect("the Bears resolved");
    assert_eq!(pt(&engine, object), (2, 2));
    let def = baylee_cards::by_index(bears).expect("in the pool");
    assert!(def.abilities.is_empty() && def.keywords.is_empty());
}
