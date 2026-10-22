//! `cards/sorceries/mv_3/political_trickery.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "e0eaa43f-1c46-4299-9ecd-5f4a32d292cd"
fn political_trickery() -> CardIndex {
    card_index("e0eaa43f-1c46-4299-9ecd-5f4a32d292cd")
}

#[test]
fn political_trickery_exchanges_control_of_your_land_and_an_opponents_land() {
    let mut engine = lk::game(
        SEED,
        &[plains()],
        &[mountain()],
        &[political_trickery()],
        &[],
    );
    let my_plains = lk::obj(&engine, P0, plains());
    let opp_mountain = lk::obj(&engine, P1, mountain());

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 2), (ManaColor::Blue, 1)],
    );
    cast_with_floating(&mut engine, P0, political_trickery());
    lk::aim(&mut engine, &[my_plains], &[]);
    lk::aim(&mut engine, &[opp_mountain], &[]);
    lk::settle(&mut engine);

    assert_eq!(engine.state().object(opp_mountain).unwrap().controller, P0);
    assert_eq!(engine.state().object(my_plains).unwrap().controller, P1);
}
