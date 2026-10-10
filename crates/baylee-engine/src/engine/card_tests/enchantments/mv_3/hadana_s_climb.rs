//! `cards/enchantments/mv_3/hadana_s_climb.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn hadana_s_climb_puts_plus_one_counter_on_controlled_creature_at_combat() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(107, forest())
        .battlefield(0, &[hadana_s_climb(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("Llanowar Elves on battlefield");
    assert_eq!(pt(&engine, elf), (1, 1), "starts as a 1/1");
    assert_eq!(
        counters_on(&engine, elf, CounterKind::P1P1),
        0,
        "starts with 0 counters"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(
        options.contains(&elf),
        "the controlled creature is a legal target"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, elf, CounterKind::P1P1),
        1,
        "one +1/+1 counter placed on the target"
    );
    assert_eq!(pt(&engine, elf), (2, 2), "elf is now 2/2");

    let climb =
        on_battlefield(&engine, p0, hadana_s_climb()).expect("Hadana's Climb on battlefield");
    assert_eq!(
        engine.state().object(climb).map(|o| o.face_index),
        Some(0),
        "Hadana's Climb remains on face 0"
    );
    assert!(
        types(&engine, climb).contains(TypeSet::ENCHANTMENT),
        "Hadana's Climb is an enchantment"
    );
}

#[test]
fn hadanas_climb_transforms_after_the_targets_third_counter() {
    let p0 = PlayerId::new(0);
    for before in [1, 2] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[hadana_s_climb(), llanowar_elves()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let source = on_battlefield(&e, p0, hadana_s_climb()).unwrap();
        let elf = on_battlefield(&e, p0, llanowar_elves()).unwrap();
        crate::replacement::put_counters(
            e.dev_state_mut(p0).unwrap(),
            elf,
            CounterKind::P1P1,
            before,
        );
        pass_until_targets(&mut e, p0);
        aim_at(&mut e, p0, elf);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(counters_on(&e, elf, CounterKind::P1P1), before + 1);
        assert_eq!(
            e.state().object(source).unwrap().face_index,
            u8::from(before == 2)
        );
    }
}
