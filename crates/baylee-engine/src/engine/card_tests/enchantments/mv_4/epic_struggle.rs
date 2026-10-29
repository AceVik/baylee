//! `cards/enchantments/mv_4/epic_struggle.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn epic_struggle_wins_the_game_at_your_upkeep_with_twenty_creatures() {
    let struggle = card_index("6f34ee2c-6e0b-4c71-9bdd-6ac55a323849");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut board = vec![forest(); 4];
    board.extend(std::iter::repeat_n(grizzly_bears(), 20));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .hand(0, &[struggle])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, struggle);
    pass_until(&mut engine, stack_is_empty);

    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| e.state().game_end.is_some());
    assert_eq!(
        engine.state().game_end,
        Some(crate::win::GameResult {
            winner: Some(crate::win::Victor::Player(p0)),
            reason: crate::win::EndReason::EffectWin,
        })
    );
}
