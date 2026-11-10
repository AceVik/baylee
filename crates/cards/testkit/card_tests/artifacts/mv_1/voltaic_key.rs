//! `cards/artifacts/mv_1/voltaic_key.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Voltaic Key` prints `{{1}}, {{T}}: Untap target artifact.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Voltaic Key`, a `quiet_artifact()`, and a `forest()`.
/// Tapping the other mana sources leaves the `quiet_artifact()` tapped while floating mana.
/// Activating `Voltaic Key` targets the tapped artifact, taps the key, and untaps the target artifact upon resolution.
#[test]
fn voltaic_key_untaps_target_artifact() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), voltaic_key(), quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let key = on_battlefield(&engine, p0, voltaic_key()).expect("key is on battlefield");
    let rock =
        on_battlefield(&engine, p0, quiet_artifact()).expect("quiet artifact is on battlefield");

    tap_all_mana_but(&mut engine, p0, Some(voltaic_key()));
    assert!(
        is_tapped(&engine, rock),
        "quiet artifact is tapped for mana"
    );
    assert!(!is_tapped(&engine, key), "key is still untapped");

    activate(&mut engine, p0, voltaic_key(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&rock), "quiet artifact is a legal target");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![rock],
            },
        )
        .expect("target chosen");

    pass_until(&mut engine, stack_is_empty);

    assert!(!is_tapped(&engine, rock), "target artifact is now untapped");
    assert!(is_tapped(&engine, key), "`Voltaic Key` is tapped");
}
