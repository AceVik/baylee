//! `cards/sorceries/mv_1/stream_of_life.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Stream of Life: "Target player gains X life."
#[test]
fn stream_of_life_gains_x_life_for_its_target_player() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[stream_of_life()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let before = life_of(&engine, p0);
    cast_from_hand(&mut engine, p0, stream_of_life());
    engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
    engine.apply(p0, PlayerAction::ChoosePlayer(p0)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, p0), before + 2, "\"gains X life\"");
}
