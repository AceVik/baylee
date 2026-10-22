//! `cards/artifacts/mv_3/misers_cage.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P1: PlayerId = PlayerId::new(1);

// oracle_id = "72a1e724-eb32-4781-b44d-3b1ed02a5a8b"
fn misers_cage() -> CardIndex {
    card_index("72a1e724-eb32-4781-b44d-3b1ed02a5a8b")
}

#[test]
fn misers_cage_deals_2_damage_at_opponents_upkeep_if_they_have_five_or_more_cards_in_hand() {
    let mut engine = lk::game(
        SEED,
        &[misers_cage()],
        &[],
        &[],
        &[plains(), plains(), plains(), plains(), plains()],
    );

    reach_their_main_phase(&mut engine, P1);

    assert_eq!(
        engine.state().players[1].life,
        18,
        "Misers' Cage dealt 2 damage to P1 at their upkeep because they had 5+ cards in hand"
    );
}
