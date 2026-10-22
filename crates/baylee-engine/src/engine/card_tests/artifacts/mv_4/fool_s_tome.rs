//! `cards/artifacts/mv_4/fool_s_tome.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn fool_s_tome() -> CardIndex {
    card_index("65589538-d3c1-4dfd-a328-71f57f556212")
}

/// Fool's Tome (`65589538-d3c1-4dfd-a328-71f57f556212`) — {4} Artifact — Book:
/// "{2}, {T}: Draw a card. Activate only if you have no cards in hand."
#[test]
fn fool_s_tome_draws_a_card_only_when_hand_is_empty() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[fool_s_tome(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(P0)).len(), 0);
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, fool_s_tome(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(P0)).len(), 1);
}
