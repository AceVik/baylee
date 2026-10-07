//! `cards/lands/utility/maze_s_end.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Maze's End prints `This land enters tapped.`, `{{T}}: Add {{C}}.`, and `{{3}}, {{T}}, Return this land to its owner's hand: Search your library for a Gate card, put it onto the battlefield, then shuffle. If you control ten or more Gates with different names, you win the game.`
///
/// Under `Coverage::Partial`, the win-the-game clause is omitted because no `Effect` variant wins the game.
/// Playing Maze's End from hand enters tapped.
/// In the next turn, with `{{3}}` floating from three `forest()` lands, activating ability 1 pays `{{3}}`, taps and returns Maze's End to hand,
/// prompts a library search via `ChoicePrompt::SearchLibrary`, and puts a `basilisk_gate()` onto the battlefield without triggering a game win.
#[test]
fn maze_s_end_enters_tapped_and_tutors_gate_to_battlefield() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, basilisk_gate())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[maze_s_end()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let maze = play_land(&mut engine, p0, maze_s_end());
    assert!(entered_tapped(&engine, maze), "maze's end enters tapped");

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, maze));

    // Float {{3}} from Forests while keeping Maze's End untapped.
    tap_mana_except(&mut engine, p0, maze);
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);

    activate(&mut engine, p0, maze_s_end(), 1);

    // Cost paid: Maze's End returned to hand, {{3}} mana spent.
    assert!(
        in_hand(&engine, p0, maze_s_end()).is_some(),
        "maze's end returned to hand as cost"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::SearchLibrary,
                ..
            }
        )
    });

    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected library search, got {:?}", engine.pending());
    };
    assert!(!options.is_empty(), "gate card available in library");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, basilisk_gate()).is_some(),
        "gate put onto battlefield"
    );
    assert!(
        !matches!(engine.pending(), Pending::GameOver(_)),
        "win-the-game clause omitted under `Coverage::Partial`"
    );
}
