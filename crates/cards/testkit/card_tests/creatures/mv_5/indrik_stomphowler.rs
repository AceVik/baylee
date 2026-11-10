//! `cards/creatures/mv_5/indrik_stomphowler.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Indrik Stomphowler` is a 4/4 creature costing `{4}{G}` under `Coverage::Implemented`.
/// It prints "When this creature enters, destroy target artifact or enchantment."
/// Upon entering the battlefield, its arrival trigger fires, targeting an artifact or enchantment
/// (such as `Basilisk Collar`) while excluding creatures (such as `Desert Drake`), and destroys the chosen target.
#[test]
fn indrik_stomphowler_destroys_target_artifact_or_enchantment() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .hand(0, &[indrik_stomphowler()])
        .battlefield(1, &[basilisk_collar(), desert_drake()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let collar =
        on_battlefield(&engine, p1, basilisk_collar()).expect("opponent controls Basilisk Collar");
    let drake =
        on_battlefield(&engine, p1, desert_drake()).expect("opponent controls Desert Drake");

    cast_from_hand(&mut engine, p0, indrik_stomphowler());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets prompt for Indrik Stomphowler, got {:?}",
            engine.pending()
        );
    };
    assert!(
        options.contains(&collar),
        "artifact is a legal target: {options:?}"
    );
    assert!(
        !options.contains(&drake),
        "creature is not an artifact or enchantment: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![collar],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    let beast = on_battlefield(&engine, p0, indrik_stomphowler())
        .expect("Indrik Stomphowler is on the battlefield");
    assert_eq!(pt(&engine, beast), (4, 4), "body is 4/4");
    assert!(
        in_graveyard(&engine, p1, basilisk_collar()).is_some(),
        "targeted artifact was destroyed and put into opponent's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, basilisk_collar()).is_none(),
        "targeted artifact is no longer on the battlefield"
    );
}
