//! `cards/lands/artifacts/tree_of_tales.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Tree of Tales prints one line — "{T}: Add {G}" — on an Artifact Land,
/// which makes it the pool's cleanest #159 board: nothing about the card is
/// a shortcut, so its tap arrives as an ordinary `(source, index)` entry in
/// `LegalActions::abilities` and *not* in `mana_abilities`, the list that
/// only ever carries the CR 305.6 basics and granted mana abilities. Playing
/// the land for real (so the untapped arrival is the game's and not the
/// harness' placement) and pressing that entry has to leave exactly one
/// green in the pool with nothing on the stack (CR 605.3b) and the permanent
/// tapped — a colour question that never comes, because a fixed `{G}` is not
/// a choice.
#[test]
fn tree_of_tales_is_an_artifact_land_whose_printed_tap_makes_one_green() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(7, forest()).hand(0, &[tree_of_tales()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, tree_of_tales());
    let kinds = types(&engine, land);
    assert!(
        kinds.contains(TypeSet::LAND) && kinds.contains(TypeSet::ARTIFACT),
        "\"Artifact Land\" is both types at once: {kinds:?}"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(land, 0)),
        "the printed {{T}}: Add {{G}} is an ordinary `(source, index)` entry: {:?}",
        legal.abilities
    );
    assert!(
        !legal.mana_abilities.contains(&land),
        "`mana_abilities` carries the CR 305.6 basics and granted abilities, \
         and this land is neither: {:?}",
        legal.mana_abilities
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing is floating before the tap"
    );

    activate(&mut engine, p0, tree_of_tales(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1, "one green, as printed");
    assert_eq!(pool.total(), 1, "and nothing else came with it");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
}
