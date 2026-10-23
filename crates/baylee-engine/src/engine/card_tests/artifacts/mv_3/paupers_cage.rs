//! `cards/artifacts/mv_3/paupers_cage.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P1: PlayerId = PlayerId::new(1);

// oracle_id = "38ec5b13-b143-4818-9243-b1715dab3bef"
fn paupers_cage() -> CardIndex {
    card_index("38ec5b13-b143-4818-9243-b1715dab3bef")
}

#[test]
fn paupers_cage_deals_2_damage_on_opponents_upkeep_when_hand_has_two_or_fewer_cards() {
    let mut engine = lk::game(SEED, &[paupers_cage()], &[], &[], &[forest(), forest()]);

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(
        lk::life(&engine, P1),
        18,
        "Paupers' Cage deals 2 damage to opponent with 2 cards in hand at upkeep"
    );
}

#[test]
fn paupers_cage_does_not_trigger_when_opponent_has_three_or_more_cards_in_hand() {
    let mut engine = lk::game(
        SEED,
        &[paupers_cage()],
        &[],
        &[],
        &[forest(), forest(), forest()],
    );

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(
        lk::life(&engine, P1),
        20,
        "Paupers' Cage does not trigger when opponent has 3 cards in hand at upkeep"
    );
}

/// "Each opponent's upkeep": the controller's own empty hand at their own
/// upkeep is not hit, and an opponent holding three cards is not either.
#[test]
fn paupers_cage_does_not_hit_its_own_controller() {
    const P0: PlayerId = PlayerId::new(0);
    let mut engine = lk::game(
        SEED,
        &[paupers_cage()],
        &[],
        &[],
        &[forest(), forest(), forest()],
    );

    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    assert_eq!(
        lk::life(&engine, P0),
        20,
        "P0 held no cards at its own upkeep, and the cage only watches opponents"
    );
    assert_eq!(lk::life(&engine, P1), 20);
}
