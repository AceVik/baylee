//! `cards/sorceries/mv_1/gamble.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Gamble — {R} sorcery: "Search your library for a card, put that card into your
/// hand, discard a card at random, then shuffle."
///
/// Discarding a card at random is the `Coverage::Partial` gap because no random discard
/// effect exists in the DSL. This scenario proves the search half: casting Gamble offers
/// a mandatory search (min: 1, max: 1) from the library directly into the player's
/// hand, and the card stays in hand without being discarded.
#[test]
fn gamble_searches_library_for_a_card_to_hand_without_random_discard() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain()])
        .hand(0, &[gamble()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let lib_size_before = library_size(&engine, p0);
    let spell = in_hand(&engine, p0, gamble()).expect("gamble is in hand");

    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .expect("one Mountain pays {R} for Gamble");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until only stops on ChooseCards")
    };
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::SearchLibrary,
        "search prompt indicates library tutor"
    );
    assert_eq!(
        (min, max),
        (1, 1),
        "Gamble searches for exactly one card (mandatory search)"
    );
    assert!(
        !options.is_empty(),
        "library has basic Forests to search from"
    );

    let chosen = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .expect("choosing the card from library is legal");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(chosen).map(|o| o.zone),
        Some(Zone::Hand),
        "the searched card arrived in hand"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        1,
        "hand holds exactly the tutored card and no random discard occurred"
    );
    assert_eq!(
        library_size(&engine, p0),
        lib_size_before - 1,
        "library is one card smaller after tutoring"
    );
    assert_eq!(
        in_graveyard(&engine, p0, gamble()),
        Some(spell),
        "Gamble resolved into the graveyard"
    );
}
