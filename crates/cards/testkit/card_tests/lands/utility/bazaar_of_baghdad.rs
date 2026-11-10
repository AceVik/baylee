//! `cards/lands/utility/bazaar_of_baghdad.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Bazaar of Baghdad: "{T}: Draw two cards, then discard three cards."
/// Activating Bazaar of Baghdad draws two cards from the library and then requires
/// discarding three cards, leaving the hand one card smaller and the land tapped.
#[test]
fn bazaar_of_baghdad_draws_two_and_discards_three() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(71, forest())
        .battlefield(0, &[bazaar_of_baghdad()])
        // Three to discard needs three to discard from: the kit deals no
        // opening hand, so without this the engine clamps the choice to the
        // two cards Bazaar itself drew and the test measures the clamp.
        .hand(0, &[forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bazaar = on_battlefield(&engine, p0, bazaar_of_baghdad()).expect("Bazaar deployed");
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let lib_before = library_size(&engine, p0);
    let gy_before = engine.state().zones.list(ZoneLocation::Graveyard(p0)).len();

    activate(&mut engine, p0, bazaar_of_baghdad(), 0);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected discard choice, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (3, 3));
    assert_eq!(prompt, ChoicePrompt::Generic);

    let discards: Vec<ObjectId> = options.into_iter().take(3).collect();
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: discards })
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(library_size(&engine, p0), lib_before - 2, "drew two cards");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        gy_before + 3,
        "discarded three cards"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1,
        "net hand size decreased by one"
    );
    assert!(is_tapped(&engine, bazaar));
}
