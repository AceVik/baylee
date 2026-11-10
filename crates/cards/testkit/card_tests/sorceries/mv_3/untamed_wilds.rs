//! `cards/sorceries/mv_3/untamed_wilds.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Untamed Wilds — `{2}{G}` sorcery: "Search your library for a basic land
/// card, put that card onto the battlefield, then shuffle."
///
/// The destinations a tutor can use are told apart only by playing it: a
/// `Wish`-style search fills the hand, Wayfarer's Bauble puts its land down
/// *tapped*, and this one does neither, so the test reads the found card's
/// zone, the battlefield's land count, the library it left and the hand it
/// did not enter. The menu is read with it, because "basic land" is a filter
/// and the filler deck is all Forests — the options have to be basic lands
/// rather than merely cards. And the resolution is walked past the search to
/// the quiet priority behind it, so the shuffle is an effect that finished
/// rather than a question left hanging.
#[test]
fn untamed_wilds_puts_the_basic_land_it_finds_onto_the_battlefield_untapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[untamed_wilds()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lands_before = lands_of(&engine, p0).len();
    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // Three Forests pay `{2}{G}`; the whole spell is three mana, so the card
    // is castable off what is on the table and nothing has to be floated in
    // advance for a claim about the offer.
    cast_from_hand(&mut engine, p0, untamed_wilds());
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
        ChoicePrompt::SearchLibrary,
        "a tutor's own question, and not a scry or a discard"
    );
    assert!(!options.is_empty(), "the library holds basic lands to find");

    let library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    for id in &options {
        assert!(
            library.contains(id),
            "\"search your library\": {id:?} is no card in it"
        );
        let chars = engine
            .state()
            .object(*id)
            .expect("a card in the library is an object")
            .characteristics();
        assert!(
            chars.types.contains(TypeSet::LAND),
            "the filter says land, and {id:?} is none"
        );
        assert!(
            chars.supertypes.contains(SupertypeSet::BASIC),
            "and basic: {id:?} is not"
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

    assert_eq!(
        engine
            .state()
            .object(chosen)
            .expect("the found card is still an object")
            .zone,
        Zone::Battlefield,
        "\"put that card onto the battlefield\" — the destination the card \
         prints, and not the hand a different tutor would have used"
    );
    assert!(
        !is_tapped(&engine, chosen),
        "nothing on the card says tapped: the land arrives standing"
    );
    assert_eq!(
        lands_of(&engine, p0).len(),
        lands_before + 1,
        "the battlefield gained exactly the one land the search found"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1,
        "the sorcery left the hand and no card entered it: a search that put \
         its find into hand would read one higher here"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"then shuffle\": the found card left the library, and shuffling \
         reorders what is left without changing how much of it there is"
    );
    assert!(
        in_graveyard(&engine, p0, untamed_wilds()).is_some(),
        "and the sorcery itself resolved into its owner's graveyard"
    );
}
