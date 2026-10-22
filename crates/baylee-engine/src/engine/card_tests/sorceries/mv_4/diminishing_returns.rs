//! `cards/sorceries/mv_4/diminishing_returns.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn diminishing_returns() -> CardIndex {
    card_index("f9b02a7c-c122-4282-90e2-81fcfa663193")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// `Diminishing Returns` ({2}{U}{U}, Sorcery, `Coverage::Partial`) shuffles
/// each player's hand and graveyard into their library and draws each player
/// seven cards.
#[test]
fn diminishing_returns_shuffles_hands_and_graveyards_and_draws_seven_each() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(); 4])
        .hand(0, &[diminishing_returns(), grizzly_bears()])
        .hand(1, &[hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    seed_graveyard(&mut engine, p0, 2);
    seed_graveyard(&mut engine, p1, 3);

    cast_from_hand(&mut engine, p0, diminishing_returns());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        1,
        "only the resolving Diminishing Returns is in P0's graveyard"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        0,
        "P1's graveyard was shuffled into P1's library"
    );
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(p0)).len(), 7);
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(p1)).len(), 7);
}
