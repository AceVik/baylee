//! `cards/instants/mv_4/refreshing_rain.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn refreshing_rain_target_player_gains_six_life() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[card_index("c6f69168-71c2-41a2-ab95-cfd7baf0ba68")])
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(
        &mut engine,
        p0,
        card_index("c6f69168-71c2-41a2-ab95-cfd7baf0ba68"),
    );
    engine
        .apply(p0, PlayerAction::ChoosePlayer(p1))
        .expect("target p1");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 26);
    assert_eq!(engine.state().players[0].life, 20);
}
