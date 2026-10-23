//! `cards/sorceries/mv_2/mulch.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn mulch() -> CardIndex {
    card_index("12c03a03-36bf-487e-b8be-e55399525da5")
}

/// Mulch (`12c03a03-36bf-487e-b8be-e55399525da5`) — {1}{G} Sorcery (Implemented):
/// "Reveal the top four cards of your library. Put all land cards revealed this way into your hand and the rest into your graveyard."
#[test]
fn mulch_puts_revealed_lands_into_hand_and_rest_into_graveyard() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[mulch()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(P0)).len();
    cast_from_hand(&mut engine, P0, mulch());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_before - 1 + 4,
        "Mulch should put all four revealed Forests from the top of the library into P0's hand"
    );
}
