//! `cards/lands/restricted/library_of_alexandria.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Library of Alexandria: "{T}: Add {C}." / "{T}: Draw a card. Activate only
/// if you have **exactly** seven cards in hand."
///
/// "Exactly" is the whole card, and it is asserted in both directions off
/// one board: six in hand is not seven, and the draw that seven buys takes
/// the hand to eight and out of range of the land that gave it. Nothing
/// here is a cost — both lines charge `{T}` alone — so what moves is the
/// offer.
#[test]
fn library_of_alexandria_draws_at_exactly_seven_cards_and_never_again() {
    let p0 = PlayerId::new(0);
    let hand_of = |n: usize, seed: u64| {
        let mut engine = Duel::new(seed, forest())
            .battlefield(0, &[library_of_alexandria()])
            .hand(0, &vec![forest(); n])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        engine
    };

    let six = hand_of(6, 1191);
    let lib = on_battlefield(&six, p0, library_of_alexandria()).expect("Library deployed");
    assert_eq!(
        six.state().zones.list(ZoneLocation::Hand(p0)).len(),
        6,
        "seat zero takes no turn-one draw (CR 103.8a), so the hand is the one dealt"
    );
    let Pending::Priority { legal, .. } = six.pending().clone() else {
        panic!("expected priority, got {:?}", six.pending());
    };
    assert!(
        legal.abilities.contains(&(lib, 0)),
        "the mana ability charges the same {{T}} and is offered"
    );
    assert!(
        !legal.abilities.contains(&(lib, 1)),
        "six is not exactly seven: {:?}",
        legal.abilities
    );

    let mut seven = hand_of(7, 1192);
    let lib = on_battlefield(&seven, p0, library_of_alexandria()).expect("Library deployed");
    let library_before = library_size(&seven, p0);
    let Pending::Priority { legal, .. } = seven.pending().clone() else {
        panic!("expected priority, got {:?}", seven.pending());
    };
    assert!(legal.abilities.contains(&(lib, 1)), "seven is seven");

    activate(&mut seven, p0, library_of_alexandria(), 1);
    pass_until(&mut seven, stack_is_empty);
    assert_eq!(library_size(&seven, p0), library_before - 1);
    assert_eq!(
        seven.state().zones.list(ZoneLocation::Hand(p0)).len(),
        8,
        "the card it drew is what takes the hand out of range"
    );
    assert!(is_tapped(&seven, lib));
}
