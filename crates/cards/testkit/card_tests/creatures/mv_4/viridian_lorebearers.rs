//! `cards/creatures/mv_4/viridian_lorebearers.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Viridian Lorebearers` is a 3/3 creature costing `{3}{G}` under `Coverage::Implemented`.
/// It prints "{3}{G}, {T}: Target creature gets +X/+X until end of turn, where X is the number of artifacts your opponents control."
/// When activated targeting itself while the opponent controls two artifacts (such as `Lightning Greaves` and `Basilisk Collar`),
/// it gets +2/+2 until end of turn, becoming a 5/5.
#[test]
fn viridian_lorebearers_pumps_target_by_opponent_artifact_count() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                viridian_lorebearers(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .battlefield(1, &[lightning_greaves(), basilisk_collar()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let lorebearers = on_battlefield(&engine, p0, viridian_lorebearers())
        .expect("Viridian Lorebearers is on the battlefield");
    assert_eq!(pt(&engine, lorebearers), (3, 3), "base body is 3/3");

    tap_all_mana_but(&mut engine, p0, Some(viridian_lorebearers()));
    activate(&mut engine, p0, viridian_lorebearers(), 0);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets prompt for Viridian Lorebearers, got {:?}",
            engine.pending()
        );
    };
    assert!(
        options.contains(&lorebearers),
        "lorebearers is a valid target: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![lorebearers],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, lorebearers),
        (5, 5),
        "target gets +2/+2 based on opponent's two controlled artifacts"
    );
    assert!(
        is_tapped(&engine, lorebearers),
        "Viridian Lorebearers is tapped from paying its activation cost"
    );
}
