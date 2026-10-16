//! `cards/creatures/artifacts/mv_3/sylvok_replica.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn sylvok_replica() -> CardIndex {
    card_index("97818ed4-3590-4a3b-92ce-716f9cfdc78e")
}

/// Sylvok Replica — "{G}, Sacrifice this creature: Destroy target artifact or
/// enchantment." The menu holds the opponent's artifact and not their
/// creature.
#[test]
fn sylvok_replica_sacrifices_to_destroy_an_artifact() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[sylvok_replica(), forest()])
        .battlefield(1, &[quiet_artifact(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let replica = on_battlefield(&engine, p0, sylvok_replica()).expect("seated");
    assert_eq!(pt(&engine, replica), (1, 3));
    let ring = on_battlefield(&engine, p1, quiet_artifact()).expect("seated");
    let creature = on_battlefield(&engine, p1, quiet_creature()).expect("seated");
    tap_all_mana(&mut engine, p0);

    activate(&mut engine, p0, sylvok_replica(), 0);
    let options = aim_at(&mut engine, p0, ring);
    assert!(options.contains(&ring));
    assert!(!options.contains(&creature), "a plain creature is neither");
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, sylvok_replica()).is_some(), "sacrificed");
    assert!(in_graveyard(&engine, p1, quiet_artifact()).is_some(), "destroyed");
    assert!(on_battlefield(&engine, p1, quiet_creature()).is_some());
}
