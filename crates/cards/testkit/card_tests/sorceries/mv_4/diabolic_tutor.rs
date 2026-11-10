//! `cards/sorceries/mv_4/diabolic_tutor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "14589b6b-1814-46f9-a364-83cc15dacac2"

/// Diabolic Tutor — {2}{B}{B} sorcery: "Search your library for a card, put
/// that card into your hand, then shuffle."
///
/// Three claims and each needs a different reading. The word "a card" is the
/// one worth building a board for: the library is sixty Counterspells, so a
/// menu that holds the whole library and still offers something is
/// `Filter::Any` and not a basic-land search — over a deck of Swamps the two
/// readings would be indistinguishable. The card the search names is then
/// followed *by object* rather than by printing, because the opening hand
/// already holds copies of the same card; and the library is exactly one
/// shorter afterwards, which is the tutor leaving it plus a shuffle that only
/// reorders what is left. The {2}{B}{B} is a real payment read off the pool
/// four Swamps filled, and the sorcery itself finishes in its owner's
/// graveyard.
#[test]
fn diabolic_tutor_searches_the_library_for_any_card_and_shuffles_it_into_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, counterspell())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[diabolic_tutor()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Mana before the claim: `legal.castable` is filtered through the pool,
    // and four Swamps are exactly {2}{B}{B}.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        4,
        "four Swamps tapped, four black"
    );
    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    cast_with_floating(&mut engine, p0, diabolic_tutor());
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
        unreachable!("pass_until stops on nothing else")
    };
    assert_eq!(player, p0, "the caster does the searching");
    assert_eq!(
        prompt,
        ChoicePrompt::SearchLibrary,
        "the tutor's own question, and not a scry or a discard"
    );
    assert!(
        !options.is_empty(),
        "\"a card\" is every card in the library, and every card in this one \
         is a nonland: a filter narrowed to lands would have offered nothing"
    );
    assert_eq!(
        options.len(),
        library_size(&engine, p0),
        "the whole library is the menu: {options:?}"
    );

    let chosen = options[0];
    let in_library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    assert!(
        in_library.contains(&chosen),
        "\"search your library\": {chosen:?} is no card in it"
    );
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
        "\"put that card into your hand\": the very object the search offered, \
         and not another copy of the same printing"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the tutor left the hand and the searched card took its place"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"then shuffle\": the found card left the library, and shuffling \
         reorders what is left without changing how much of it there is"
    );
    assert!(
        in_graveyard(&engine, p0, diabolic_tutor()).is_some(),
        "a sorcery that has resolved goes to its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}}{{B}}{{B}} it charges came out of the pool"
    );
}
