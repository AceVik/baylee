//! `cards/artifacts/mv_2/galvanic_key.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Galvanic Key` prints `Flash` and `{{3}}, {{T}}: Untap target artifact.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Galvanic Key`, a `quiet_artifact()`, and three copies of `forest()`.
/// `Galvanic Key` possesses `KeywordSet::FLASH`. Tapping the other mana sources leaves the `quiet_artifact()` tapped.
/// Activating `Galvanic Key` pays three mana to target and untap the tapped artifact upon resolution.
#[test]
fn galvanic_key_has_flash_and_untaps_target_artifact() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                galvanic_key(),
                quiet_artifact(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let key = on_battlefield(&engine, p0, galvanic_key()).expect("key on battlefield");
    let rock = on_battlefield(&engine, p0, quiet_artifact()).expect("artifact on battlefield");
    assert!(
        keywords(&engine, key).contains(KeywordSet::FLASH),
        "`Galvanic Key` has flash"
    );

    tap_all_mana_but(&mut engine, p0, Some(galvanic_key()));
    assert!(is_tapped(&engine, rock), "rock tapped for mana");
    assert!(!is_tapped(&engine, key), "key is untapped");

    activate(&mut engine, p0, galvanic_key(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets prompt, got {:?}", engine.pending());
    };
    assert!(options.contains(&rock), "artifact is a legal target");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![rock],
            },
        )
        .expect("targeted artifact");

    pass_until(&mut engine, stack_is_empty);

    assert!(!is_tapped(&engine, rock), "target artifact was untapped");
    assert!(is_tapped(&engine, key), "`Galvanic Key` is tapped");
}
