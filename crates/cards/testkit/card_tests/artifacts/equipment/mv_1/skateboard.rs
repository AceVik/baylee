//! `cards/artifacts/equipment/mv_1/skateboard.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn skateboard() -> CardIndex {
    card_index("decfde5e-ea53-4b15-8f69-51da7ef39d15")
}

/// Skateboard — "When this Equipment enters, tap target permanent. Equipped
/// creature gets +1/+0 and has haste." and "Equip {1}". Cast for {1}: the
/// trigger asks for any permanent (an opposing Elf is chosen and taps), then
/// the Elf wearing it is a 2/1 with haste.
#[test]
fn skateboard_taps_a_permanent_and_grants_haste() {
    let elf = llanowar_elves();
    let mut engine = rig(2, &[skateboard()], &[elf], &[elf]);
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mine = on_battlefield(&engine, p0, elf).expect("my Elf");
    let theirs = on_battlefield(&engine, p1, elf).expect("their Elf");

    float(&mut engine, 1);
    cast_with_floating(&mut engine, p0, skateboard());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert!(options.contains(&theirs), "any permanent, theirs");
    assert!(options.contains(&mine), "any permanent, mine");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("their Elf is the target");
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, theirs), "tapped by the trigger");
    assert!(!is_tapped(&engine, mine));

    equip(&mut engine, skateboard(), 1, mine);
    assert_eq!(pt(&engine, mine), (2, 1));
    assert!(keywords(&engine, mine).contains(KeywordSet::HASTE));
    assert!(!keywords(&engine, theirs).contains(KeywordSet::HASTE));
}
