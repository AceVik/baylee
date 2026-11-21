//! `cards/instants/mv_2/sonic_burst.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "144d42b8-5c44-4a36-9028-75229047e56c"
fn sonic_burst() -> CardIndex {
    card_index("144d42b8-5c44-4a36-9028-75229047e56c")
}

/// Sonic Burst ({1}{R}): cannot be cast if you have no other card in hand to
/// discard at random as its additional cost.
#[test]
fn sonic_burst_cannot_be_cast_as_sole_card_in_hand() {
    let mut engine = lk::game(SEED, &[], &[], &[sonic_burst()], &[]);
    lk::float(&mut engine, P0, &[(ManaColor::Red, 2)]);
    assert!(
        !lk::castable(&engine, P0, sonic_burst()),
        "cannot pay the additional random discard with an otherwise empty hand"
    );
}

/// With another card in hand, casting Sonic Burst immediately discards one
/// card at random as a cost (before resolution) and deals 4 damage to any target.
#[test]
fn sonic_burst_discards_one_card_at_random_as_cost_and_deals_four_damage() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[index::HILL_GIANT],
        &[sonic_burst(), sonic_burst(), mountain(), plains()],
        &[],
    );
    let giant = lk::obj(&engine, P1, index::HILL_GIANT);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(P0)).len();

    lk::float(&mut engine, P0, &[(ManaColor::Red, 4)]);
    cast_with_floating(&mut engine, P0, sonic_burst());
    lk::aim(&mut engine, &[], &[P1]);

    // While Sonic Burst is still on the stack, 1 random card is already in GY.
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_before - 2,
        "spell left hand AND 1 card was discarded at random as an additional cost"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(P0)).len(),
        1,
        "discarded card is in graveyard before resolution"
    );

    lk::settle(&mut engine);
    assert_eq!(lk::life(&engine, P1), 16, "4 damage to target player");

    // Second Sonic Burst can also target a creature (if it wasn't the one discarded).
    let mut creature_game = lk::game(
        SEED,
        &[],
        &[index::HILL_GIANT],
        &[sonic_burst(), mountain()],
        &[],
    );
    let target_giant = lk::obj(&creature_game, P1, index::HILL_GIANT);
    lk::float(&mut creature_game, P0, &[(ManaColor::Red, 2)]);
    lk::cast_at(&mut creature_game, P0, sonic_burst(), &[target_giant], &[]);

    assert!(
        in_graveyard(&creature_game, P1, index::HILL_GIANT).is_some(),
        "4 damage destroys a 3/3 creature"
    );
    assert!(
        in_graveyard(&creature_game, P0, mountain()).is_some(),
        "the sole other card in hand was discarded"
    );
    assert_eq!(on_battlefield(&engine, P1, index::HILL_GIANT), Some(giant));
}
