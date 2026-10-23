//! `cards/instants/mv_2/final_fortune.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn final_fortune() -> CardIndex {
    card_index("8d0adc5c-3fdd-4e22-b783-5651f8e57b65")
}

/// Final Fortune (8d0adc5c-3fdd-4e22-b783-5651f8e57b65) — {R}{R} Instant:
/// "Take an extra turn after this one. At the beginning of that turn's end
/// step, you lose the game."
///
/// Cast in turn 1: P0 survives the end of turn 1, takes turn 2 (the opponent
/// does not), is still in the game in that turn's main phase, and loses at the
/// beginning of its end step, so P1 wins.
#[test]
fn final_fortune_grants_an_extra_turn_and_loses_the_game_at_its_end_step() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain()])
        .hand(0, &[final_fortune()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, final_fortune());
    pass_until(&mut engine, stack_is_empty);

    // The extra turn: turn 2 is P0's, not the opponent's, and P0 is alive in it.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 2 && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(engine.state().turn.active, P0);
    assert!(
        !matches!(engine.pending(), Pending::GameOver(_)),
        "no loss before that turn's end step"
    );

    // Its end step: P0 loses, and the game is not carried on into turn 3.
    pass_until(&mut engine, |e| matches!(e.pending(), Pending::GameOver(_)));
    let Pending::GameOver(result) = engine.pending().clone() else {
        unreachable!("pass_until stopped on the end of the game")
    };
    assert_eq!(result.winner, Some(crate::win::Victor::Player(P1)));
    assert_eq!(engine.state().turn.number, 2, "lost in the extra turn");
    assert_eq!(engine.state().turn.phase, Phase::Ending);
}
