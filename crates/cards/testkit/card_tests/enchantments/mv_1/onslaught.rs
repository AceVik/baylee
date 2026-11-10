//! `cards/enchantments/mv_1/onslaught.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Onslaught` (`Coverage::Implemented`):
/// "Whenever you cast a creature spell, tap target creature."
///
/// Verifies that casting a creature spell triggers `Onslaught`, prompting for a target
/// creature to tap, and that upon resolution the targeted creature becomes tapped.
#[test]
fn onslaught_taps_target_creature_on_casting_creature_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1323, forest())
        .battlefield(0, &[onslaught(), forest()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their elf deployed");
    assert!(!is_tapped(&engine, their_elf));

    cast_from_hand(&mut engine, p0, llanowar_elves());

    // Onslaught triggers on casting a creature spell
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for Onslaught trigger, got {:?}",
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
        is_tapped(&engine, their_elf),
        "targeted creature was tapped by Onslaught"
    );
}
