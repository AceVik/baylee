//! `cards/lands/utility/maze_s_end.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::win::{EndReason, GameResult, Victor};

/// Maze's End prints `This land enters tapped.`, `{T}: Add {C}.`, and `{3}, {T}, Return this land to its owner's hand: Search your library for a Gate card, put it onto the battlefield, then shuffle. If you control ten or more Gates with different names, you win the game.`
///
/// Playing Maze's End from hand enters tapped.
/// In the next turn, with `{3}` floating from three `forest()` lands, activating ability 1 pays `{3}`, taps and returns Maze's End to hand,
/// prompts a library search via `ChoicePrompt::SearchLibrary`, and puts a `basilisk_gate()` onto the battlefield. One Gate is nowhere near
/// ten Gates with different names, so the game goes on: the win clause is played by the tests below this one.
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
        "one Gate is not ten Gates with different names: the game is not won"
    );
}

/// Nine Gates, each with a different name: every Ravnica guildgate but the
/// Simic one. With the Simic Guildgate found by the search, that is ten.
fn nine_different_gates() -> Vec<CardIndex> {
    vec![
        azorius_guildgate(),
        boros_guildgate(),
        dimir_guildgate(),
        golgari_guildgate(),
        gruul_guildgate(),
        izzet_guildgate(),
        orzhov_guildgate(),
        rakdos_guildgate(),
        selesnya_guildgate(),
    ]
}

/// A game in `p0`'s first main phase: Maze's End, three Forests and `mine`
/// on `p0`'s side, `theirs` on the opponent's, a library of nothing but
/// `library` -- and the three Forests tapped, so `{3}` floats and Maze's End
/// (and every Gate, which would tap for mana too) stays untapped.
#[track_caller]
fn maze_game(
    mine: &[CardIndex],
    theirs: &[CardIndex],
    library: CardIndex,
) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut board = vec![maze_s_end(), forest(), forest(), forest()];
    board.extend_from_slice(mine);
    let mut engine = Duel::new(SEED, library)
        .battlefield(0, &board)
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let forests = all_on_battlefield(&engine, p0, forest());
    tap_mana_where(&mut engine, p0, |id| forests.contains(&id));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the Forests pay the {{3}}"
    );
    engine
}

/// Activates Maze's End's search and answers what the search asks the way a
/// player would: takes the first Gate on offer, or none when the menu is
/// empty. Returns once the ability has resolved or the game is over.
#[track_caller]
fn search_for_a_gate(engine: &mut Engine<RegistryLookup>, p0: PlayerId) {
    activate(engine, p0, maze_s_end(), 1);
    for _ in 0..10 {
        pass_until(engine, |e| {
            stack_is_empty(e)
                || matches!(
                    e.pending(),
                    Pending::ChooseCards { .. } | Pending::GameOver(_)
                )
        });
        match engine.pending().clone() {
            Pending::ChooseCards {
                player, options, ..
            } => {
                assert_eq!(player, p0, "its controller searches");
                let objects = options.into_iter().take(1).collect();
                engine
                    .apply(p0, PlayerAction::ChooseObjects { objects })
                    .unwrap();
            }
            _ => return,
        }
    }
    panic!("Maze's End never finished asking");
}

/// The result of a finished game.
#[track_caller]
fn the_result(engine: &Engine<RegistryLookup>) -> GameResult {
    let Pending::GameOver(result) = engine.pending().clone() else {
        panic!("the game is not over, it asks {:?}", engine.pending())
    };
    result
}

/// `{T}: Add {C}.` -- the plain mana ability, ability 0: one colourless mana
/// from an untapped Maze's End, and the land is tapped for it.
#[test]
fn maze_s_end_taps_for_one_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[maze_s_end()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let maze = on_battlefield(&engine, p0, maze_s_end()).expect("Maze's End is seated");

    activate(&mut engine, p0, maze_s_end(), 0);

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1,
        "{{T}}: Add {{C}}"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);
    assert!(is_tapped(&engine, maze), "{{T}} taps it");
}

/// "If you control ten or more Gates with different names, you win the game."
/// Nine different Gates are on the battlefield and the library holds the
/// tenth: the search finds it, Maze's End is back in hand (it was part of
/// the cost), and the game is won by its controller.
#[test]
fn maze_s_end_wins_the_game_with_ten_gates_of_different_names() {
    let p0 = PlayerId::new(0);
    let mut engine = maze_game(&nine_different_gates(), &[], simic_guildgate());

    search_for_a_gate(&mut engine, p0);

    assert!(
        in_hand(&engine, p0, maze_s_end()).is_some(),
        "\"Return this land to its owner's hand\" was the cost"
    );
    assert!(
        on_battlefield(&engine, p0, simic_guildgate()).is_some(),
        "the search put the tenth Gate onto the battlefield"
    );
    let result = the_result(&engine);
    assert_eq!(
        result.winner,
        Some(Victor::Player(p0)),
        "ten Gates with different names: its controller wins"
    );
    assert_eq!(result.reason, EndReason::EffectWin, "won by an effect");
}

/// "Controlling multiple Gates with the same name has no effect on your
/// ability to win the game with Maze's End. The excess Gates are simply
/// ignored." (ruling, 2013-04-15.) Nine different Gates plus a second
/// Azorius Guildgate from the library are ten Gates with only nine names:
/// no win, and the game goes on.
#[test]
fn maze_s_end_does_not_win_with_ten_gates_sharing_a_name() {
    let p0 = PlayerId::new(0);
    let mut engine = maze_game(&nine_different_gates(), &[], azorius_guildgate());

    search_for_a_gate(&mut engine, p0);

    assert_eq!(
        all_on_battlefield(&engine, p0, azorius_guildgate()).len(),
        2,
        "the search found a second Azorius Guildgate: ten Gates on the table"
    );
    assert!(
        !matches!(engine.pending(), Pending::GameOver(_)),
        "but only nine names, and the excess Gate is ignored: the game is not won"
    );
    assert!(
        in_hand(&engine, p0, maze_s_end()).is_some(),
        "Maze's End is still in hand after the cost"
    );
}

/// "You'll search for a Gate and put it onto the battlefield before checking
/// to see if you win the game. This check will happen even if you don't put a
/// Gate onto the battlefield this way." (ruling, 2013-04-15.) All ten
/// guildgates are already on the battlefield and the library holds no Gate at
/// all, so the search finds nothing, and the game is won all the same.
#[test]
fn maze_s_end_checks_for_the_win_even_when_the_search_finds_nothing() {
    let p0 = PlayerId::new(0);
    let mut gates = nine_different_gates();
    gates.push(simic_guildgate());
    let mut engine = maze_game(&gates, &[], forest());

    search_for_a_gate(&mut engine, p0);

    let result = the_result(&engine);
    assert_eq!(
        result.winner,
        Some(Victor::Player(p0)),
        "ten Gates with different names were already there when the ability resolved"
    );
}

/// "You control": only Gates its controller controls count. Eight different
/// Gates are on `p0`'s side, the search finds a ninth, and two more
/// different Gates stand on the *opponent's* side -- eleven names at the
/// table, nine under `p0`'s control, so no win.
#[test]
fn maze_s_end_counts_only_the_gates_its_controller_controls() {
    let p0 = PlayerId::new(0);
    let mine = [
        azorius_guildgate(),
        boros_guildgate(),
        dimir_guildgate(),
        golgari_guildgate(),
        gruul_guildgate(),
        izzet_guildgate(),
        orzhov_guildgate(),
        rakdos_guildgate(),
    ];
    let theirs = [selesnya_guildgate(), basilisk_gate()];
    let mut engine = maze_game(&mine, &theirs, simic_guildgate());

    search_for_a_gate(&mut engine, p0);

    assert!(
        on_battlefield(&engine, p0, simic_guildgate()).is_some(),
        "the search found its ninth Gate"
    );
    assert!(
        !matches!(engine.pending(), Pending::GameOver(_)),
        "the opponent's Gates are not ones its controller controls: nine, not ten"
    );
}

/// "This check will happen only as the ability resolves, not at other
/// times." (ruling, 2013-04-15.) Ten Gates with different names and Maze's End
/// all sit on the battlefield through a whole turn cycle and nobody wins:
/// the clause is part of the activated ability, not a state trigger.
#[test]
fn maze_s_end_does_not_win_by_merely_sitting_beside_ten_gates() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut board = nine_different_gates();
    board.push(simic_guildgate());
    board.push(maze_s_end());
    let mut engine = Duel::new(SEED, forest()).battlefield(0, &board).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(
        !matches!(engine.pending(), Pending::GameOver(_)),
        "nothing resolved, so nothing was checked: {:?}",
        engine.pending()
    );
    assert!(
        on_battlefield(&engine, p0, maze_s_end()).is_some(),
        "Maze's End never left the battlefield"
    );
}
