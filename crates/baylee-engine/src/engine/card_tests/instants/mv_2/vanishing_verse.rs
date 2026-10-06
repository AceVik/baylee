//! `cards/instants/mv_2/vanishing_verse.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Vanishing Verse is a `{W}{B}` instant whose whole text is "Exile target
/// monocolored permanent", and "monocolored" names a **count of colours**
/// rather than a type line: an object with exactly one colour in it qualifies
/// (CR 105.2c), so a gold permanent and a colourless one are both off the menu.
///
/// The defending board therefore carries all three readings at once — a green
/// Llanowar Elves, a black-and-green Lotleth Troll and a colourless Sol Ring —
/// and the test reads both the menu and the move: the one permanent that was a
/// legal target leaves for *exile*, which a spell that had quietly resolved
/// against nothing could not show.
#[test]
fn vanishing_verse_exiles_the_monocolored_permanent_and_leaves_the_other_two_alone() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), swamp(), swamp()])
        .hand(0, &[vanishing_verse()])
        .battlefield(1, &[llanowar_elves(), lotleth_troll(), quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elf is out");
    let troll = on_battlefield(&engine, p1, lotleth_troll()).expect("the Troll is out");
    let ring = on_battlefield(&engine, p1, quiet_artifact()).expect("the Sol Ring is out");

    cast_from_hand(&mut engine, p0, vanishing_verse());
    let options = pass_until_targets(&mut engine, p0);
    assert_eq!(
        options,
        vec![elf],
        "\"monocolored permanent\" on a board holding a green creature, a \
         black-and-green one and a colourless artifact: one colour is the whole menu"
    );
    assert!(
        !options.contains(&troll),
        "two colours is not one, whatever the other creature is: {options:?}"
    );
    assert!(
        !options.contains(&ring),
        "and no colour at all is not one either (CR 105.2c): {options:?}"
    );

    let refused = engine.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![troll],
        },
    );
    assert!(
        refused.is_err(),
        "an answer the question did not enumerate is refused: {refused:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the permanent the question offered is the one it exiles");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "the targeted permanent left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_none(),
        "and exile is not the graveyard, which is where a destroy would have put it"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p1))
            .iter()
            .copied()
            .any(|id| engine
                .state()
                .object(id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == llanowar_elves()))),
        "\"exile\": the card is in exile under its owner, not merely off the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, vanishing_verse()).is_some(),
        "the instant itself resolved rather than being countered or fizzling"
    );
    assert!(
        on_battlefield(&engine, p1, lotleth_troll()).is_some()
            && on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "the two permanents the filter declined never moved"
    );
}
