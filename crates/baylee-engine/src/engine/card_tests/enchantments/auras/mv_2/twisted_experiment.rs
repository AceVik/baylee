//! `cards/enchantments/auras/mv_2/twisted_experiment.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Twisted Experiment` (`Coverage::Implemented`):
/// "Enchant creature. Enchanted creature gets +3/-1."
///
/// Verifies that enchanting a creature with `Twisted Experiment` modifies its
/// power by +3 and toughness by -1, while leaving bystander creatures unchanged.
#[test]
fn twisted_experiment_modifies_power_and_toughness() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1303, forest())
        .battlefield(0, &[swamp(), forest(), a_seven_five_wurm()])
        .battlefield(1, &[a_seven_five_wurm()])
        .hand(0, &[twisted_experiment()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, a_seven_five_wurm()).expect("my wurm deployed");
    let theirs = on_battlefield(&engine, p1, a_seven_five_wurm()).expect("their wurm deployed");
    assert_eq!(pt(&engine, mine), (7, 5));

    cast_from_hand(&mut engine, p0, twisted_experiment());
    let Pending::ChooseTargets { .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for Aura spell, got {:?}",
            engine.pending()
        )
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, twisted_experiment()).expect("Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(mine),
        "Aura attached to the chosen wurm"
    );
    assert_eq!(
        pt(&engine, mine),
        (10, 4),
        "7/5 wurm gets +3/-1 to become 10/4"
    );
    assert_eq!(
        pt(&engine, theirs),
        (7, 5),
        "the wurm across the table is untouched"
    );
}
