//! `cards/creatures/artifacts/mv_3/elf_replica.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Elf Replica` prints `{{1}}{{G}}, Sacrifice this creature: Destroy target enchantment.`
/// with `Coverage::Implemented`.
/// In this scenario, seat 0 floats two mana from two `forest()` lands to activate the ability,
/// targeting the opponent's `their_enchantment()`. Choosing the target concludes the announcement,
/// paying the sacrifice cost, and resolving the ability sends the enchantment to the graveyard.
#[test]
fn elf_replica_sacrifices_to_destroy_target_enchantment() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), elf_replica()])
        .battlefield(1, &[their_enchantment()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ench = on_battlefield(&engine, p1, their_enchantment()).expect("target enchantment seated");
    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);

    activate(&mut engine, p0, elf_replica(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for enchantment, got {:?}",
            engine.pending()
        );
    };
    assert!(options.contains(&ench));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ench],
            },
        )
        .unwrap();

    // After target declaration, the sacrifice cost is paid.
    assert!(in_graveyard(&engine, p0, elf_replica()).is_some());
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, their_enchantment()).is_none(),
        "the enchantment was destroyed"
    );
    assert!(
        in_graveyard(&engine, p1, their_enchantment()).is_some(),
        "the destroyed enchantment lies in the opponent's graveyard"
    );
}
