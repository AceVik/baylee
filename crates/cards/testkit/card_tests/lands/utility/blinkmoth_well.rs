//! `cards/lands/utility/blinkmoth_well.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Blinkmoth Well` is a utility land under `Coverage::Implemented` that taps for `{C}` and can pay `{2}` and tap to tap target noncreature artifact.
/// When its second ability is activated, the target filter restricts choices to noncreature artifacts, excluding artifact creatures and non-artifacts.
/// Upon resolution, the targeted artifact becomes tapped while Blinkmoth Well remains tapped from paying its cost.
#[test]
fn blinkmoth_well_taps_target_noncreature_artifact() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), blinkmoth_well()])
        .battlefield(1, &[lightning_greaves(), myr_retriever(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let well = on_battlefield(&engine, p0, blinkmoth_well()).expect("Blinkmoth Well deployed");
    let greaves = on_battlefield(&engine, p1, lightning_greaves()).expect("Greaves deployed");
    let retriever = on_battlefield(&engine, p1, myr_retriever()).expect("Retriever deployed");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("Elf deployed");

    assert!(!is_tapped(&engine, greaves), "Greaves starts untapped");

    // Float {2} from the two Forests while keeping Blinkmoth Well untapped.
    tap_all_mana_but(&mut engine, p0, Some(blinkmoth_well()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Forests produce two mana"
    );

    // Ability 0 is mana ability {T}: Add {C}; ability 1 is {2}, {T}: Tap target noncreature artifact.
    activate(&mut engine, p0, blinkmoth_well(), 1);

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert_eq!(player, p0, "activating player chooses target");
    assert_eq!((min, max), (1, 1), "exactly one target required");
    assert!(
        options.contains(&greaves),
        "noncreature artifact Greaves is an offered target: {options:?}"
    );
    assert!(
        !options.contains(&retriever),
        "artifact creature is excluded by the noncreature filter: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "non-artifact creature is excluded: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![greaves],
            },
        )
        .expect("targeting Greaves is legal");

    assert!(
        is_tapped(&engine, well),
        "Blinkmoth Well tapped as activation cost"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{2}} spent from mana pool"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        is_tapped(&engine, greaves),
        "targeted Greaves was tapped by the ability"
    );
}
