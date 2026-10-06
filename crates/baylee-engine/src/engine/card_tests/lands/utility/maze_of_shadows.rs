//! `cards/lands/utility/maze_of_shadows.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Maze of Shadows prints two abilities and only the first one is written:
/// `{T}: Add {C}`, while the untap-a-shadow-attacker line is the
/// `Coverage::Partial` gap, since shadow is not an enforced keyword bit. The
/// scenario plays it as the turn's land drop and then taps it beside two
/// Forests, which is the reading that separates the two lists `LegalActions`
/// keeps: a basic land type would land in `mana_abilities` by the CR 305.6
/// shortcut, and a nonbasic's own printed `{T}` only ever arrives as an
/// indexed entry in `abilities` (#159). One colourless in a pool of two green
/// is a number no other source on that board could have produced.
#[test]
fn maze_of_shadows_arrives_untapped_and_taps_for_one_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(713, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[maze_of_shadows()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let maze = play_land(&mut engine, p0, maze_of_shadows());
    assert!(
        !entered_tapped(&engine, maze),
        "the Maze prints no enters-tapped clause, and a real `PlayLand` is \
         what lets the entry be read at all"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "playing a land pays for nothing"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "the seat holds priority after the land drop, got {:?}",
            engine.pending()
        )
    };
    assert!(
        legal.abilities.contains(&(maze, 0)),
        "the one line the card carries is offered: {:?}",
        legal.abilities
    );
    assert_eq!(
        legal
            .abilities
            .iter()
            .filter(|(source, _)| *source == maze)
            .count(),
        1,
        "and the untap-and-prevent line, which shadow could not be filtered \
         for, is not a second button: {:?}",
        legal.abilities
    );

    let taken = tap_all_mana(&mut engine, p0);
    assert_eq!(taken, 3, "two Forests and the Maze's own {{T}}");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "{{T}}: Add {{C}} — the half a basic-type shortcut could never see"
    );
    assert_eq!(pool.available(ManaColor::Green), 2, "the Forests beside it");
    assert_eq!(pool.total(), 3, "and nothing else came with them");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, maze), "the Maze paid its own {{T}}");
}
