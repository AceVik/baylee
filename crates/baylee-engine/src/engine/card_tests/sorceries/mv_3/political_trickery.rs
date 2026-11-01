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

/// "Target land you control and target land an opponent controls": each
/// menu holds only its own side, and "indefinitely" outlasts the turn.
#[test]
fn political_trickery_menus_are_one_side_each_and_the_exchange_lasts() {
    let mut engine = lk::game(
        SEED,
        &[plains(), island()],
        &[mountain(), forest()],
        &[political_trickery()],
        &[],
    );
    let my_plains = lk::obj(&engine, P0, plains());
    let my_island = lk::obj(&engine, P0, island());
    let opp_mountain = lk::obj(&engine, P1, mountain());
    let opp_forest = lk::obj(&engine, P1, forest());

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 2), (ManaColor::Blue, 1)],
    );
    cast_with_floating(&mut engine, P0, political_trickery());
    let mine = aim_at(&mut engine, P0, my_plains);
    assert!(mine.contains(&my_plains) && mine.contains(&my_island));
    assert!(
        !mine.contains(&opp_mountain) && !mine.contains(&opp_forest),
        "the first target is a land you control"
    );
    let theirs = aim_at(&mut engine, P0, opp_mountain);
    assert!(theirs.contains(&opp_mountain) && theirs.contains(&opp_forest));
    assert!(
        !theirs.contains(&my_plains) && !theirs.contains(&my_island),
        "the second target is a land an opponent controls"
    );
    lk::settle(&mut engine);
    assert_eq!(engine.state().object(opp_mountain).unwrap().controller, P0);
    assert_eq!(engine.state().object(my_plains).unwrap().controller, P1);
    assert_eq!(
        engine.state().object(my_island).unwrap().controller,
        P0,
        "only the two chosen lands changed hands"
    );
    assert_eq!(engine.state().object(opp_forest).unwrap().controller, P1);

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(
        engine.state().object(opp_mountain).unwrap().controller,
        P0,
        "indefinitely"
    );
}
