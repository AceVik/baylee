//! `cards/artifacts/mv_4/razor_pendulum.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "bf7c16db-01c9-41ed-bda6-c5326198ce77"
fn razor_pendulum() -> CardIndex {
    card_index("bf7c16db-01c9-41ed-bda6-c5326198ce77")
}

#[test]
fn razor_pendulum_deals_2_damage_at_end_step_when_active_player_has_five_or_less_life() {
    let mut engine = lk::game(SEED, &[razor_pendulum()], &[], &[], &[]);
    engine.dev_state_mut(P0).unwrap().players[0].life = 5;

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(lk::life(&engine, P0), 3);
    assert_eq!(lk::life(&engine, P1), 20);
}
