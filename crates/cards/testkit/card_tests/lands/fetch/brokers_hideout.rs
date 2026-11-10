//! `cards/lands/fetch/brokers_hideout.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Brokers Hideout: "When this land enters, sacrifice it. When you do,
/// search your library for a basic Forest, Plains, or Island card, put it
/// onto the battlefield tapped, then shuffle and you gain 1 life." The
/// second sentence is a reflexive triggered ability (CR 603.12), created by
/// the sacrifice. It used to be the land's own `Trigger::LeavesBattlefield`,
/// which also fired on a bounce in response. CR 603.12's Manticore example
/// rules that reading out, and `reflexive_tests` plays it.
///
/// The filter is checked by the branch that finds **nothing**: with a deck
/// of Mountains the search has no legal card, and Mountain is a basic land
/// — so a card written as "search for a basic land" would still find one
/// here. Both engines gain the life, because "you gain 1 life" is a
/// separate clause and not a consequence of the search.
#[test]
fn brokers_hideout_sacrifices_itself_and_finds_one_of_the_three_types_it_names() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[brokers_hideout()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let life_before = engine.state().players[0].life;
    let library_before = library_size(&engine, p0);
    play_land(&mut engine, p0, brokers_hideout());

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
        panic!("the search asks which land, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the land's controller searches");
    assert_eq!(prompt, ChoicePrompt::SearchLibrary);
    assert!(!options.is_empty(), "this deck is made of Forests");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .expect("a card the search offered");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        on_battlefield(&engine, p0, brokers_hideout()).is_none(),
        "\"sacrifice it\" — the land it entered as is gone"
    );
    assert!(
        in_graveyard(&engine, p0, brokers_hideout()).is_some(),
        "and it went to its owner's graveyard"
    );
    let found = on_battlefield(&engine, p0, forest()).expect("the Forest it searched up");
    assert!(
        is_tapped(&engine, found),
        "\"put it onto the battlefield tapped\""
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "one card left the library, and the shuffle moved none"
    );
    assert_eq!(
        engine.state().players[0].life,
        life_before + 1,
        "\"and you gain 1 life\""
    );

    // The other branch: a deck of Mountains is a deck of basic lands this
    // card may not find, so the search asks nothing at all.
    let mut lean = Duel::new(SEED, mountain())
        .hand(0, &[brokers_hideout()])
        .start();
    keep_mulligans(&mut lean);
    assert!(walk_to_own_main(&mut lean, p0), "p0 reaches its own main");
    let lean_life = lean.state().players[0].life;
    let lean_library = library_size(&lean, p0);
    play_land(&mut lean, p0, brokers_hideout());
    pass_until(&mut lean, |e| at_rest(e, p0));

    assert_eq!(
        library_size(&lean, p0),
        lean_library,
        "a Mountain is a basic land and is not a Forest, Plains or Island"
    );
    assert!(
        in_graveyard(&lean, p0, brokers_hideout()).is_some(),
        "the sacrifice does not depend on the search finding anything"
    );
    assert_eq!(
        lean.state().players[0].life,
        lean_life + 1,
        "and neither does the life, which is its own clause"
    );
}
