//! `cards/sorceries/mv_5/psychic_transfer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "5554d422-6513-4c7e-866d-9d5f0403fc80"
fn psychic_transfer() -> CardIndex {
    card_index("5554d422-6513-4c7e-866d-9d5f0403fc80")
}

#[test]
fn psychic_transfer_exchanges_life_totals_with_target_player() {
    let mut engine = lk::game(SEED, &[], &[], &[psychic_transfer()], &[]);
    engine.dev_state_mut(P0).unwrap().players[0].life = 15;
    engine.dev_state_mut(P0).unwrap().players[1].life = 20;

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 4), (ManaColor::Blue, 1)],
    );
    cast_with_floating(&mut engine, P0, psychic_transfer());
    engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    lk::settle(&mut engine);

    assert_eq!(lk::life(&engine, P0), 20);
    assert_eq!(lk::life(&engine, P1), 15);
}
