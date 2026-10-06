//! `cards/creatures/artifacts/mv_3/nim_replica.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Nim Replica` prints `{{2}}{{B}}, Sacrifice this creature: Target creature gets -1/-1 until end of turn.`
/// with `Coverage::Implemented`.
/// In this scenario, seat 0 floats three black mana from three `swamp()` lands to activate the ability,
/// targeting the opponent's 1/1 `quiet_creature()`. Choosing the target concludes the announcement,
/// paying the sacrifice cost, and resolving the ability reduces the elf's toughness to zero so that
/// state-based actions send it to the graveyard.
#[test]
fn nim_replica_sacrifices_to_give_target_creature_minus_one_minus_one() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), nim_replica()])
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p1, quiet_creature()).expect("opponent creature seated");
    assert_eq!(pt(&engine, elf), (1, 1));

    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);

    activate(&mut engine, p0, nim_replica(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for creature, got {:?}",
            engine.pending()
        );
    };
    assert!(options.contains(&elf));
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();

    // After target declaration, the sacrifice cost is paid.
    assert!(in_graveyard(&engine, p0, nim_replica()).is_some());
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, quiet_creature()).is_none(),
        "the 1/1 elf died from having 0 toughness"
    );
    assert!(
        in_graveyard(&engine, p1, quiet_creature()).is_some(),
        "the elf is now in the graveyard"
    );
}
