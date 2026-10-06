//! `cards/sorceries/mv_3/fabricate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fabricate — {2}{U} sorcery: "Search your library for an artifact card,
/// reveal it, put it into your hand, then shuffle."
///
/// The backing deck is a pile of Sol Rings rather than the usual Forests,
/// because the filter is `Filter::ARTIFACT`: over basic lands the search
/// offers nothing and an empty menu would satisfy "it looked at the library"
/// while the card found no card at all. The found object is followed by
/// identity and not by printing — the seat has drawn Sol Rings in its opening
/// hand already, so "a Sol Ring is in hand" was true before the spell
/// resolved and proves nothing. The library count is read with it because
/// "then shuffle" reorders what is left without changing how much of it there
/// is, and the hand is one card up only if the card actually moved.
#[test]
fn fabricate_tutors_an_artifact_card_out_of_the_library_into_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, quiet_artifact())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[fabricate()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let library_before = library_size(&engine, p0);
    cast_from_hand(&mut engine, p0, fabricate());
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
    assert_eq!(
        player, p0,
        "the seat that cast the spell does the searching"
    );
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::SearchLibrary,
        "the tutor's own question, and not a scry or a discard"
    );
    assert!(
        !options.is_empty(),
        "the library holds artifact cards to find"
    );

    // The spell has already left the hand for the stack, so this is the hand
    // the tutored card is about to join.
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let in_library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    for id in &options {
        assert!(
            in_library.contains(id),
            "\"search your library\": {id:?} is no card in it"
        );
        assert!(
            types(&engine, *id).contains(TypeSet::ARTIFACT),
            "\"an artifact card\": {id:?} is not one, so the filter was skipped"
        );
    }

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
        "the very card the search offered reached the hand, and not another \
         copy of the same printing"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "one card up — a reveal that left the card where it was could not do \
         this"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"then shuffle\": the found card left the library, and shuffling \
         reorders what is left without changing how much of it there is"
    );
    assert!(
        in_graveyard(&engine, p0, fabricate()).is_some(),
        "the sorcery resolved and went to its owner's graveyard"
    );
}
