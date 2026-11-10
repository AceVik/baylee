//! `cards/creatures/mv_5/flowstone_overseer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Flowstone Overseer` is a 4/4 creature under `Coverage::Implemented` with an activated ability costing `{R}{R}`.
/// Activating its ability requires `{R}{R}` in the mana pool and targets a creature on the battlefield.
/// Upon resolution, the targeted creature gets +1/-1 until end of turn while the Overseer itself does not tap.
#[test]
fn flowstone_overseer_pumps_target_creature_power_and_reduces_toughness() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), flowstone_overseer()])
        .battlefield(1, &[rootbreaker_wurm()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let overseer = on_battlefield(&engine, p0, flowstone_overseer()).expect("Overseer deployed");
    let wurm = on_battlefield(&engine, p1, rootbreaker_wurm()).expect("Wurm deployed");

    assert_eq!(pt(&engine, overseer), (4, 4), "Overseer is 4/4");
    assert_eq!(pt(&engine, wurm), (6, 6), "Wurm starts as 6/6");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.contains(&(overseer, 0)),
        "cannot afford {{R}}{{R}} with an empty mana pool"
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        2,
        "two Mountains provide {{R}}{{R}}"
    );

    activate(&mut engine, p0, flowstone_overseer(), 0);

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
        options.contains(&wurm) && options.contains(&overseer),
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

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        0,
        "costs were paid upon choosing targets"
    );
    assert!(
        !is_tapped(&engine, overseer),
        "the ability does not require tapping the Overseer"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, wurm),
        (7, 5),
        "Wurm received +1/-1, becoming 7/5"
    );
    assert_eq!(
        pt(&engine, overseer),
        (4, 4),
        "untargeted Overseer remains 4/4"
    );
}
