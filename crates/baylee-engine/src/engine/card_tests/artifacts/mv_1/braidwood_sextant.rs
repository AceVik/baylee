//! `cards/artifacts/mv_1/braidwood_sextant.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Braidwood Sextant — {1} artifact: "{2}, {T}, Sacrifice this artifact:
/// Search your library for a basic land card, reveal that card, put it into
/// your hand, then shuffle."
///
/// All three parts of the price are read in one activation and each one is
/// visible in a different place: the {2} leaves the pool, the Sextant leaves
/// the battlefield, and the card goes to its owner's graveyard — where a
/// sacrifice goes and not where an exile would. The tutor half is read as a
/// *move* and not as a question that was asked: the card the search offered
/// is the card now in hand, the library is one shorter for it (a shuffle
/// moves what is left without changing how much of it there is), and the hand
/// grew by exactly one. Three Forests pay the cast and the ability both, so
/// the pool holds the {2} before the claim, and the Forests are left tapped
/// while nothing else on the board can make mana — the Sextant is an
/// artifact, so `tap_all_mana` never spends it.
#[test]
fn braidwood_sextant_eats_itself_for_a_basic_land_out_of_the_library() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(881, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[braidwood_sextant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Three Forests into the pool: {1} for the artifact, and the {2} the
    // ability then charges are already floating beside it.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Forests, and the Sextant makes no mana of its own"
    );
    cast_with_floating(&mut engine, p0, braidwood_sextant());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, braidwood_sextant()).is_some(),
        "the artifact resolved onto the table"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the {{1}} is spent and the ability's {{2}} is still in the pool"
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // Ability 0 is the only line the card prints.
    activate(&mut engine, p0, braidwood_sextant(), 0);
    assert!(
        on_battlefield(&engine, p0, braidwood_sextant()).is_none(),
        "`Sacrifice this artifact` is paid on announcement (CR 601.2h)"
    );
    assert!(
        in_graveyard(&engine, p0, braidwood_sextant()).is_some(),
        "and the card is in its owner's graveyard, not merely gone"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}} came out of the pool"
    );

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
    assert_eq!(player, p0, "the seat that activated does the searching");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::SearchLibrary,
        "the tutor's own question, and not a scry or a discard"
    );
    assert!(!options.is_empty(), "the library holds basic lands to find");
    let in_library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    for id in &options {
        assert!(
            in_library.contains(id),
            "\"search your library\": {id:?} is no card in it"
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
        "\"put that card into your hand\": the very card the search offered, \
         and not some other copy of the same printing"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "one card up, which a reveal that left the card where it was could not do"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"then shuffle\": the found card left the library, and shuffling \
         reorders what is left without changing how much of it there is"
    );
    assert!(
        on_battlefield(&engine, p0, braidwood_sextant()).is_none(),
        "the Sextant is gone — the same artifact cannot tutor twice"
    );
}
