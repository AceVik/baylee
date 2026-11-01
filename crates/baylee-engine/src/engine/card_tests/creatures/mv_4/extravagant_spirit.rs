//! Tests for Extravagant Spirit.

#[allow(clippy::wildcard_imports)]
use super::*;

// oracle_id = "6470afc0-2f67-454a-8a5b-d8dfea4b84ce"
fn extravagant_spirit() -> CardIndex {
    card_index("6470afc0-2f67-454a-8a5b-d8dfea4b84ce")
}

/// At the beginning of your upkeep, sacrifice Extravagant Spirit unless you
/// pay {1} for each card in your hand.
#[test]
fn extravagant_spirit_upkeep_pays_per_card_in_hand_or_sacrifices() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[extravagant_spirit(), island(), island()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, extravagant_spirit());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, extravagant_spirit()).is_some());

    // Advance to P0's next upkeep (2 cards in hand -> pay {2}).
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0
            && e.state().turn.step == Step::Upkeep
            && matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, extravagant_spirit()).is_none());
}
