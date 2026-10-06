//! `cards/sorceries/mv_2/nature_s_lore.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Nature's Lore ({1}{G}) prints one sentence: search your library for a
/// **Forest card**, put it onto the battlefield, then shuffle. Two Forests pay
/// for the spell before anything is claimed about the search, and the search
/// is read as a move rather than as a question that was asked: the card the
/// offer named is the card now standing on the battlefield, the library is one
/// shorter, and the sorcery itself left the hand. The fetched land arrives
/// **untapped**, which is exactly the word the card prints and the one a
/// `Find::TAPPED` spelling would have quietly lost beside a board that already
/// had two tapped Forests on it.
#[test]
fn natures_lore_fetches_a_forest_onto_the_battlefield_untapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, basic_forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[natures_lore()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    assert_eq!(
        lands_of(&engine, p0).len(),
        2,
        "two Forests are the whole board the spell is paid with"
    );

    cast_from_hand(&mut engine, p0, natures_lore());
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
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the caster does the searching");
    assert_eq!(
        prompt,
        ChoicePrompt::SearchLibrary,
        "the tutor's own question, and not a scry or a discard"
    );
    assert_eq!(
        (min, max),
        (1, 1),
        "one card, and the search is not optional"
    );
    assert_eq!(
        options.len(),
        library_before,
        "\"a Forest card\": every card left in this library qualifies, so the \
         offer is the whole library and nothing was silently dropped from it"
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

    assert_eq!(
        engine
            .state()
            .object(chosen)
            .expect("the fetched card exists")
            .zone,
        Zone::Battlefield,
        "\"put that card onto the battlefield\""
    );
    assert!(
        engine
            .state()
            .object(chosen)
            .expect("the fetched card exists")
            .characteristics()
            .types
            .contains(TypeSet::LAND),
        "and what landed is the land the search offered"
    );
    assert!(
        !is_tapped(&engine, chosen),
        "\"onto the battlefield\" untapped — a `Find::TAPPED` spelling would \
         have put it down sideways beside the two Forests that paid"
    );
    assert_eq!(
        lands_of(&engine, p0).len(),
        3,
        "it joined the two Forests the spell was paid with"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"then shuffle\": the found card left the library, and shuffling \
         reorders what is left without changing how much of it there is"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1,
        "and the sorcery itself left the hand — a search that never resolved \
         would leave every pile exactly where it was"
    );
}
