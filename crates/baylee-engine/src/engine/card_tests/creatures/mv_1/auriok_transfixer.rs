//! `cards/creatures/mv_1/auriok_transfixer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Auriok Transfixer` prints `{{W}}, {{T}}: Tap target artifact.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Auriok Transfixer` and a Plains, while seat 1 controls an untapped artifact (`quiet_artifact()`) and a creature.
/// Activating the ability filters legal targets to artifacts, taps `Auriok Transfixer`, and upon resolution taps the opponent's artifact.
#[test]
fn auriok_transfixer_taps_target_artifact() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[auriok_transfixer(), plains()])
        .battlefield(1, &[quiet_artifact(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let scout = on_battlefield(&engine, p0, auriok_transfixer()).expect("scout seated");
    let rock = on_battlefield(&engine, p1, quiet_artifact()).expect("artifact seated");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("elf seated");

    assert!(!is_tapped(&engine, scout));
    assert!(!is_tapped(&engine, rock));

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1
    );

    activate(&mut engine, p0, auriok_transfixer(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets prompt, got {:?}", engine.pending());
    };
    assert!(
        options.contains(&rock),
        "target artifact is offered as a legal target"
    );
    assert!(
        !options.contains(&elf),
        "creature is not an artifact and cannot be targeted"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![rock],
            },
        )
        .expect("target artifact chosen");

    assert!(
        is_tapped(&engine, scout),
        "`Auriok Transfixer` tapped to pay activation cost"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        is_tapped(&engine, rock),
        "target artifact is now tapped upon resolution"
    );
}
