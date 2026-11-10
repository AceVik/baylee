//! `cards/enchantments/mv_1/seal_of_strength.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Seal of Strength` (`Coverage::Implemented`):
/// "Sacrifice this enchantment: Target creature gets +3/+3 until end of turn."
///
/// Verifies that activating `Seal of Strength` sacrifices itself and gives +3/+3
/// to the targeted creature until end of turn.
#[test]
fn seal_of_strength_pumps_target_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1326, forest())
        .battlefield(0, &[seal_of_strength(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("elf deployed");
    assert_eq!(pt(&engine, elf), (1, 1));

    activate(&mut engine, p0, seal_of_strength(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for Seal of Strength, got {:?}",
            engine.pending()
        )
    };
    assert!(options.contains(&elf));
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, elf), (4, 4), "1/1 elf gets +3/+3 to become 4/4");
    assert!(
        in_graveyard(&engine, p0, seal_of_strength()).is_some(),
        "Seal of Strength is in graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, seal_of_strength()).is_none(),
        "Seal of Strength left the battlefield"
    );
}
