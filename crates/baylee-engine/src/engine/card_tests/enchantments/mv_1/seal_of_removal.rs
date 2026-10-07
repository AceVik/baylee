//! `cards/enchantments/mv_1/seal_of_removal.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Seal of Removal` (`Coverage::Implemented`):
/// "Sacrifice this enchantment: Return target creature to its owner's hand."
///
/// Verifies that activating `Seal of Removal` sacrifices itself and returns the
/// targeted creature to its owner's hand.
#[test]
fn seal_of_removal_bounces_target_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1325, forest())
        .battlefield(0, &[seal_of_removal()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their elf deployed");

    activate(&mut engine, p0, seal_of_removal(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for Seal of Removal, got {:?}",
            engine.pending()
        )
    };
    assert!(options.contains(&their_elf));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![their_elf],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "bounced creature left the battlefield"
    );
    assert!(
        in_hand(&engine, p1, llanowar_elves()).is_some(),
        "bounced creature is in owner's hand"
    );
    assert!(
        in_graveyard(&engine, p0, seal_of_removal()).is_some(),
        "Seal of Removal is in graveyard"
    );
}
