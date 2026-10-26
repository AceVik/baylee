//! `cards/creatures/mv_3/squee_goblin_nabob.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

// oracle_id = "0ef760f4-bbec-4735-8d60-fc24d973452a"
fn squee_goblin_nabob() -> CardIndex {
    card_index("0ef760f4-bbec-4735-8d60-fc24d973452a")
}

/// `Squee, Goblin Nabob` (`Coverage::Implemented`):
/// "At the beginning of your upkeep, you may return this card from your
/// graveyard to your hand."
#[test]
fn squee_goblin_nabob_may_return_from_graveyard_to_hand_at_upkeep() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(9703, mountain())
        .battlefield(0, &[squee_goblin_nabob(), mountain()])
        .hand(0, &[index::LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let squee = on_battlefield(&engine, p0, squee_goblin_nabob()).expect("Squee on battlefield");
    cast_from_hand(&mut engine, p0, index::LIGHTNING_BOLT);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![squee],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, squee_goblin_nabob()).is_some());

    // Advance to P1's turn, then to P0's upkeep.
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && matches!(e.pending(), Pending::YesNo { .. })
    });

    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_hand(&engine, p0, squee_goblin_nabob()).is_some(),
        "Squee, Goblin Nabob returns from graveyard to hand when accepted at upkeep"
    );
    assert!(
        in_graveyard(&engine, p0, squee_goblin_nabob()).is_none(),
        "Squee, Goblin Nabob is no longer in the graveyard"
    );
}
