//! `cards/enchantments/mv_3/hadana_s_climb.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Hadana's Climb` // `Winged Temple of Orazca` (`Coverage::Partial`):
/// "At the beginning of combat on your turn, put a +1/+1 counter on target creature you control.
/// Then if that creature has three or more +1/+1 counters on it, transform `Hadana's Climb`.
/// // `{{T}}`: Add one mana of any color. `{{1}}{{G}}{{U}}`, `{{T}}`: Target creature you control
/// gains flying and gets +X/+X until end of turn, where X is its power."
///
/// Under `Coverage::Partial`, the transform clause is omitted, while the combat-begin trigger
/// placing a +1/+1 counter on a controlled creature is implemented. The test advances to the
/// combat phase, targets a controlled 1/1 `llanowar_elves()`, verifies that the counter is placed
/// and its projected power and toughness become 2/2, and confirms that `Hadana's Climb` remains
/// on face 0.
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
