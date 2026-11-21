//! `cards/instants/mv_1/sonic_seizure.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "f43c68f1-97ac-47a4-82d3-a41277259388"
fn sonic_seizure() -> CardIndex {
    card_index("f43c68f1-97ac-47a4-82d3-a41277259388")
}

/// Sonic Seizure ({R}): cannot be cast with no other card in hand to discard
/// at random.
#[test]
fn sonic_seizure_cannot_be_cast_as_sole_card_in_hand() {
    let mut engine = lk::game(SEED, &[], &[], &[sonic_seizure()], &[]);
    lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    assert!(
        !lk::castable(&engine, P0, sonic_seizure()),
        "requires another card in hand to discard at random"
    );
}

/// Casting Sonic Seizure immediately discards a random card as an additional
/// cost and deals 3 damage to any target (player or creature).
#[test]
fn sonic_seizure_discards_random_card_as_cost_and_deals_three_damage() {
    let mut player_game = lk::game(SEED, &[], &[], &[sonic_seizure(), mountain()], &[]);
    lk::float(&mut player_game, P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut player_game, P0, sonic_seizure());
    lk::aim(&mut player_game, &[], &[P1]);
    assert!(
        in_graveyard(&player_game, P0, mountain()).is_some(),
        "random discard paid as cost before resolution"
    );
    lk::settle(&mut player_game);
    assert_eq!(lk::life(&player_game, P1), 17, "3 damage to target player");

    let mut creature_game = lk::game(
        SEED,
        &[],
        &[index::HILL_GIANT],
        &[sonic_seizure(), plains()],
        &[],
    );
    let giant = lk::obj(&creature_game, P1, index::HILL_GIANT);
    lk::float(&mut creature_game, P0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut creature_game, P0, sonic_seizure(), &[giant], &[]);
    assert!(in_graveyard(&creature_game, P1, index::HILL_GIANT).is_some());
    assert!(in_graveyard(&creature_game, P0, plains()).is_some());
}
