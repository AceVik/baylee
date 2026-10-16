//! `cards/creatures/artifacts/mv_3/neurok_replica.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn neurok_replica() -> CardIndex {
    card_index("de8cdbd4-d8f7-4ce8-a69b-6d2020beff63")
}

/// Neurok Replica — 1/4; "{1}{U}, Sacrifice this creature: Return target
/// creature to its owner's hand."
#[test]
fn neurok_replica_sacrifices_to_bounce_a_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[neurok_replica(), island(), forest()])
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let replica = on_battlefield(&engine, p0, neurok_replica()).expect("seated");
    assert_eq!(pt(&engine, replica), (1, 4));
    let victim = on_battlefield(&engine, p1, quiet_creature()).expect("seated");
    tap_all_mana(&mut engine, p0);

    activate(&mut engine, p0, neurok_replica(), 0);
    aim_at(&mut engine, p0, victim);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, neurok_replica()).is_some(), "sacrificed");
    assert!(on_battlefield(&engine, p1, quiet_creature()).is_none());
    assert!(
        in_hand(&engine, p1, quiet_creature()).is_some(),
        "back in its owner's hand"
    );
}
