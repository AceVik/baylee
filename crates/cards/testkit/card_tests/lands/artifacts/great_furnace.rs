//! `cards/lands/artifacts/great_furnace.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Great Furnace prints one line, `{T}: Add {R}`, on a permanent whose type
/// line is the rest of the card: an artifact **and** a land, with no basic
/// land type. That one absence is why #159's two lists come apart here — the
/// Forest beside it is named by the CR 305.6 shortcut, while the Furnace is an
/// ordinary `(source, 0)` entry in `legal.abilities` — and the two are read
/// against each other so neither list is asserted over a board that holds one
/// permanent. The colour is what the tap is for: one red and nothing else,
/// with the Forest still standing, so the mana on the table has exactly one
/// possible source.
#[test]
fn great_furnace_is_an_artifact_land_whose_printed_tap_makes_red() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(9001, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[great_furnace()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let furnace = play_land(&mut engine, p0, great_furnace());
    let basic = on_battlefield(&engine, p0, forest()).expect("the Forest was seeded untapped");

    let kinds = types(&engine, furnace);
    assert!(
        kinds.contains(TypeSet::ARTIFACT) && kinds.contains(TypeSet::LAND),
        "an artifact land is both types at once: {kinds:?}"
    );
    assert!(
        !types(&engine, basic).contains(TypeSet::ARTIFACT),
        "and the Forest beside it is the control: a land that is not an artifact"
    );

    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!(
            "the land drop leaves p0 holding priority, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "and it is the seat that played it");
    assert!(
        legal.abilities.contains(&(furnace, 0)),
        "a printed `{{T}}: Add {{R}}` is an ordinary ability entry and not the \
         shortcut: {:?}",
        legal.abilities
    );
    assert!(
        legal.mana_abilities.contains(&basic),
        "the CR 305.6 shortcut does name the Forest beside it: {:?}",
        legal.mana_abilities
    );
    assert!(
        !legal.mana_abilities.contains(&furnace),
        "and declines the artifact, which prints no basic land type — the \
         asymmetry of #159 read off one board: {:?}",
        legal.mana_abilities
    );

    activate(&mut engine, p0, great_furnace(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        1,
        "{{T}}: Add {{R}} — one red, out of the artifact's own tap"
    );
    assert_eq!(pool.total(), 1, "and nothing else is in the pool");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(
        is_tapped(&engine, furnace),
        "the Furnace paid its own {{T}}"
    );
    assert!(
        !is_tapped(&engine, basic),
        "and the Forest beside it never moved, so the red has one source"
    );
}
