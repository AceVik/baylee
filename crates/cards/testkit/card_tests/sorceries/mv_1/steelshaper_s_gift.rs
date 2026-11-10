//! `cards/sorceries/mv_1/steelshaper_s_gift.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Steelshaper's Gift ({W} sorcery) prints one sentence: "Search your library
/// for an Equipment card, reveal that card, put it into your hand, then
/// shuffle." The library here is a stack of Lightning Greaves — an Equipment,
/// and one that costs {0}, so the same scenario can play what it fetched and
/// show the search delivered a castable card rather than a name.
///
/// `Find::HAND` is the half that separates this from Wayfarer's Bauble and
/// Journeyer's Kite, which put the card onto the battlefield instead: the
/// chosen object is read in *hand*, the library loses exactly one card to a
/// shuffle that reorders without resizing, and the Gift itself is spent into
/// the graveyard.
#[test]
fn steelshaper_s_gift_tutors_an_equipment_card_into_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, lightning_greaves())
        // Three Plains: {W} for the Gift, and the {2} the Greaves themselves
        // cost — it is the *equip* that is free, not the artifact.
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[steelshaper_s_gift()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let library_before = library_size(&engine, p0);
    cast_from_hand(&mut engine, p0, steelshaper_s_gift());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the seat that cast it does the searching");
    assert_eq!(
        prompt,
        ChoicePrompt::SearchLibrary,
        "a tutor's own question, and not a scry or a discard"
    );
    assert!(
        !options.is_empty(),
        "the library is a stack of Equipment cards to find"
    );
    let chosen = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .expect("a card the search offered is a legal answer");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&chosen),
        "\"put it into your hand\": the very card the search offered, in hand"
    );
    assert_eq!(
        engine
            .state()
            .object(chosen)
            .and_then(|o| o.card)
            .map(|c| c.index),
        Some(lightning_greaves()),
        "and it is the Equipment the library is made of, not some other card"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"then shuffle\": the found card left the library, and shuffling \
         reorders what is left without changing how much of it there is"
    );
    assert!(
        in_graveyard(&engine, p0, steelshaper_s_gift()).is_some(),
        "a sorcery that resolved is in its owner's graveyard"
    );

    // The tutor is only worth anything if what it fetched can be played:
    // the {2} left floating after the Gift is exactly the Greaves' own cost,
    // and an artifact needs a main phase its own seat holds priority in
    // (CR 117.1a).
    pass_until(&mut engine, |e| {
        stack_is_empty(e)
            && matches!(e.state().turn.phase, Phase::FirstMain | Phase::SecondMain)
            && e.state().turn.active == p0
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    cast_with_floating(&mut engine, p0, lightning_greaves());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, lightning_greaves()).is_some(),
        "the Equipment the Gift fetched is a real card: it arrived on the \
         battlefield without a single source being tapped"
    );
}
