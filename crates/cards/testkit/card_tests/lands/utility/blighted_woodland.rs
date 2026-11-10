//! `cards/lands/utility/blighted_woodland.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Blighted Woodland: "{3}{G}, {T}, Sacrifice this land: Search your library
/// for up to two basic land cards, put them onto the battlefield tapped,
/// then shuffle."
///
/// "Up to two" is the word this plays. The DSL says it as two
/// `Find::BATTLEFIELD_TAPPED` slots with `optional: true`, and the two
/// halves of that spelling fail differently: a card that found exactly two
/// would refuse a player who wanted one, and a card that found one would
/// quietly halve every copy in the pool. So the search is answered with
/// **both** cards and the count is asserted, and the offer's own bounds are
/// read to show that one was allowed.
#[test]
fn blighted_woodland_finds_up_to_two_basic_lands_tapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[blighted_woodland(), forest(), forest(), forest(), forest()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p0, blighted_woodland()).expect("the land deployed");
    let library_before = library_size(&engine, p0);
    let forests_before = all_on_battlefield(&engine, p0, forest());

    tap_mana_except(&mut engine, p0, land);
    activate(&mut engine, p0, blighted_woodland(), 1);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("the search asks which lands, got {:?}", engine.pending())
    };
    assert_eq!(
        (min, max),
        (0, 2),
        "\"up to two\" — naming none is an answer and three is not"
    );
    assert!(options.len() >= 2, "this deck is made of Forests");
    let found = vec![options[0], options[1]];
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: found })
        .expect("two cards the search offered");
    pass_until(&mut engine, |e| at_rest(e, p0));

    let arrived = all_on_battlefield(&engine, p0, forest());
    assert_eq!(
        arrived.len(),
        forests_before.len() + 2,
        "both of the two it was allowed to find arrived"
    );
    // The lands already on the board paid the {3}{G} and are tapped too, so
    // "everything is tapped" would be true of a card that put its finds down
    // untapped. The two new objects are named instead.
    let new: Vec<_> = arrived
        .iter()
        .copied()
        .filter(|id| !forests_before.contains(id))
        .collect();
    assert_eq!(new.len(), 2, "two objects that were not there before");
    assert!(
        new.iter().all(|id| is_tapped(&engine, *id)),
        "\"put them onto the battlefield tapped\""
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 2,
        "two cards left the library"
    );
    assert!(
        in_graveyard(&engine, p0, blighted_woodland()).is_some(),
        "\"Sacrifice this land\" was part of the cost"
    );
}
