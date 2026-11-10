//! `cards/creatures/mv_5/grandmother_sengir.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Grandmother Sengir` is a legendary 3/3 creature under `Coverage::Implemented` with an activated ability costing `{1}{B}` and tapping herself.
/// When activated, she targets a creature on the battlefield and taps as part of the activation cost.
/// Upon resolution, the targeted creature gets -1/-1 until end of turn while other creatures remain unaffected.
#[test]
fn grandmother_sengir_taps_and_shrinks_target_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), grandmother_sengir()])
        .battlefield(1, &[rootbreaker_wurm()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let sengir = on_battlefield(&engine, p0, grandmother_sengir()).expect("Sengir deployed");
    let wurm = on_battlefield(&engine, p1, rootbreaker_wurm()).expect("Wurm deployed");

    assert_eq!(pt(&engine, sengir), (3, 3), "Grandmother Sengir is 3/3");
    assert_eq!(pt(&engine, wurm), (6, 6), "Wurm starts as 6/6");

    // Tap Swamps for mana while keeping Grandmother Sengir untapped for her own tap cost.
    tap_mana_except(&mut engine, p0, sengir);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        2,
        "two Swamps provide {{1}}{{B}}"
    );

    activate(&mut engine, p0, grandmother_sengir(), 0);

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert_eq!(player, p0, "activating player chooses target");
    assert_eq!((min, max), (1, 1), "exactly one target required");
    assert!(
        options.contains(&wurm) && options.contains(&sengir),
        "both creatures are valid targets: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wurm],
            },
        )
        .expect("targeting Wurm is legal");

    assert!(
        is_tapped(&engine, sengir),
        "Grandmother Sengir tapped as cost"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{1}}{{B}} was spent from the mana pool"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, wurm),
        (5, 5),
        "Wurm received -1/-1, becoming 5/5"
    );
    assert_eq!(
        pt(&engine, sengir),
        (3, 3),
        "Grandmother Sengir remained 3/3"
    );
}
