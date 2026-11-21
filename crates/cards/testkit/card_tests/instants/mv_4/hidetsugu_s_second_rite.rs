//! `cards/instants/mv_4/hidetsugu_s_second_rite.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "749683c9-8cc0-4e8e-8d90-adda5e5f4e93"
fn hidetsugu_s_second_rite() -> CardIndex {
    card_index("749683c9-8cc0-4e8e-8d90-adda5e5f4e93")
}

/// Hidetsugu's Second Rite ({3}{R}): deals 0 damage if target player has 11 or
/// 9 life, and deals 10 damage if target player has EXACTLY 10 life.
#[test]
fn hidetsugus_second_rite_deals_ten_only_at_exactly_ten_life() {
    for (start_life, expected_life) in [(11, 11), (9, 9), (10, 0)] {
        let mut engine = lk::game(SEED, &[], &[], &[hidetsugu_s_second_rite()], &[]);
        engine.dev_state_mut(P0).unwrap().players[1].life = start_life;
        lk::float(&mut engine, P0, &[(ManaColor::Red, 4)]);
        cast_with_floating(&mut engine, P0, hidetsugu_s_second_rite());
        engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
        lk::settle(&mut engine);
        assert_eq!(
            lk::life(&engine, P1),
            expected_life,
            "start life {start_life} -> expected {expected_life}"
        );
    }
}
