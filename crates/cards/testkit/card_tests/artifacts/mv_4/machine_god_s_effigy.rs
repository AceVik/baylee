//! `cards/artifacts/mv_4/machine_god_s_effigy.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Machine God's Effigy` is an artifact costing `{4}` under `Coverage::Implemented`.
/// It prints "You may have this artifact enter as a copy of any creature on the battlefield,
/// except it's an artifact and it has '{T}: Add {U}.' (It's not a creature.)"
/// When cast copying `llanowar_elves()`, it enters as a noncreature artifact with `{T}: Add {U}`
/// and may tap for mana immediately because noncreatures do not have summoning sickness.
#[test]
fn machine_gods_effigy_enters_as_noncreature_artifact_copy_and_taps_for_blue() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[island(), island(), island(), island(), llanowar_elves()],
        )
        .hand(0, &[machine_god_s_effigy()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("elf deployed");

    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, p0, machine_god_s_effigy());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets prompt for CopyOnEnter, got {:?}",
            engine.pending()
        );
    };
    assert!(
        options.contains(&elf),
        "creature is an offered target: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    let effigy = on_battlefield(&engine, p0, machine_god_s_effigy())
        .expect("Machine God's Effigy entered the battlefield");
    assert_ne!(effigy, elf);

    let chars = engine
        .state()
        .object(effigy)
        .expect("object exists")
        .characteristics();
    assert!(chars.types.contains(TypeSet::ARTIFACT), "it is an artifact");
    assert!(
        !chars.types.contains(TypeSet::CREATURE),
        "it is not a creature"
    );

    // The copy retains the Elf's mana ability at index 0 and its copiable
    // exception adds the blue mana ability at index 1.
    activate(&mut engine, p0, machine_god_s_effigy(), 1);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1,
        "produced one blue mana"
    );
    assert!(is_tapped(&engine, effigy), "effigy is tapped");
}
