//! `cards/lands/utility/desolate_lighthouse.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Desolate Lighthouse: "{1}{U}{R}, {T}: Draw a card, then discard a card."
/// An Island, a Mountain, and a Forest pay the {1}{U}{R} cost to loot.
/// The controller draws one card, selects one card to discard, and the land remains tapped.
#[test]
fn desolate_lighthouse_loots_with_mana_and_tap() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(93, forest())
        .battlefield(0, &[desolate_lighthouse(), island(), mountain(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lighthouse =
        on_battlefield(&engine, p0, desolate_lighthouse()).expect("Lighthouse deployed");
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let gy_before = engine.state().zones.list(ZoneLocation::Graveyard(p0)).len();

    tap_mana_except(&mut engine, p0, lighthouse);
    activate(&mut engine, p0, desolate_lighthouse(), 1);

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
        panic!("expected discard prompt, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (1, 1));
    assert_eq!(prompt, ChoicePrompt::Generic);

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "drew one and discarded one: net hand size unchanged"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        gy_before + 1,
        "discarded card is in graveyard"
    );
    assert!(is_tapped(&engine, lighthouse));
}
