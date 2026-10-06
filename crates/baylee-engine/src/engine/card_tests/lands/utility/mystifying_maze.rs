//! `cards/lands/utility/mystifying_maze.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mystifying Maze prints `{T}: Add {C}` and a `{4}, {T}` line that exiles an
/// attacking creature and hands it back at the next end step; the card file
/// leaves the second sentence off entirely (`Coverage::Partial`), so the tap
/// is the whole card and is what is played here — as a real land drop, since
/// `starting_battlefield` places a permanent with `Cause::Setup` and a card
/// nobody watches enter is not the card.
///
/// The reading is the point. The Maze is a nonbasic with no basic land type,
/// so its printed mana ability is an ordinary `(source, index)` entry in
/// `LegalActions::abilities` and never the CR 305.6 shortcut in
/// `mana_abilities` — the half of #159 that a tap-everything helper written
/// against basics alone used to float nothing for. And the colourless mana is
/// read off the pool from an empty one, so nothing else on this board could
/// have produced it.
#[test]
fn mystifying_maze_taps_for_the_colorless_mana_it_prints() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[mystifying_maze()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    play_land(&mut engine, p0, mystifying_maze());
    let maze =
        on_battlefield(&engine, p0, mystifying_maze()).expect("the Maze landed off the drop");

    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!(
            "a land drop leaves a quiet priority: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "and the seat that played it holds it");
    assert!(
        legal.abilities.contains(&(maze, 0)),
        "the Maze's only written line is a printed mana ability: {:?}",
        legal.abilities
    );
    assert!(
        !legal.mana_abilities.contains(&maze),
        "no basic land type, so CR 305.6 has nothing to name here: {:?}",
        legal.mana_abilities
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and nothing is floating before the tap"
    );

    activate(&mut engine, p0, mystifying_maze(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "{{T}}: Add {{C}}, and nothing was named to make it"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, maze), "the Maze paid its own {{T}}");
}
