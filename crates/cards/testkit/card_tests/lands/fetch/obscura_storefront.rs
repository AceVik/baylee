//! `cards/lands/fetch/obscura_storefront.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Obscura Storefront is Brokers Hideout's sibling in a different wedge —
/// "search your library for a basic Plains, Island, or Swamp card" — and it
/// is played the same way for the same reason: the branch that finds
/// **nothing** is what separates the three types it prints from "a basic
/// land". A Forest is a basic land and is none of the three.
///
/// The search is a reflexive triggered ability (CR 603.12). It goes on the
/// stack after the sacrifice, and only if the sacrifice happened.
/// `reflexive_tests` plays the responses this scenario does not make.
#[test]
fn obscura_storefront_searches_for_one_of_the_three_basics_it_names() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[obscura_storefront()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let life_before = engine.state().players[0].life;
    play_land(&mut engine, p0, obscura_storefront());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("the search asks which land, got {:?}", engine.pending())
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .expect("a card the search offered");
    pass_until(&mut engine, |e| at_rest(e, p0));

    let found = on_battlefield(&engine, p0, island()).expect("the Island it searched up");
    assert!(
        is_tapped(&engine, found),
        "\"put it onto the battlefield tapped\""
    );
    assert!(
        in_graveyard(&engine, p0, obscura_storefront()).is_some(),
        "\"sacrifice it\" — and it went to its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].life,
        life_before + 1,
        "\"and you gain 1 life\""
    );

    // A deck of Forests: a basic land this card may not find.
    let mut lean = Duel::new(SEED, forest())
        .hand(0, &[obscura_storefront()])
        .start();
    keep_mulligans(&mut lean);
    assert!(walk_to_own_main(&mut lean, p0), "p0 reaches its own main");
    let lean_library = library_size(&lean, p0);
    play_land(&mut lean, p0, obscura_storefront());
    pass_until(&mut lean, |e| at_rest(e, p0));
    assert_eq!(
        library_size(&lean, p0),
        lean_library,
        "a Forest is a basic land and is not a Plains, Island or Swamp"
    );
}
