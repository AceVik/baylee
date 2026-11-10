//! `cards/creatures/mv_2/dwarven_miner.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Dwarven Miner` prints an activated ability with cost `{{2}}{{R}}, {{T}}`
/// targeting a nonbasic land to destroy it under `Coverage::Implemented`.
/// Across a board containing an opponent's nonbasic `Badlands` and basic `Mountain`,
/// `Pending::ChooseTargets` offers only the nonbasic land, successfully destroying it.
#[test]
fn dwarven_miner_destroys_nonbasic_land() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(1613, forest())
        .battlefield(0, &[dwarven_miner(), mountain(), mountain(), mountain()])
        .battlefield(1, &[badlands(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let nonbasic = on_battlefield(&engine, p1, badlands()).expect("Badlands is seated");
    let basic = on_battlefield(&engine, p1, mountain()).expect("Mountain is seated");

    tap_all_mana(&mut engine, p0);

    activate(&mut engine, p0, dwarven_miner(), 0);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice for Dwarven Miner");
    };
    assert!(
        options.contains(&nonbasic),
        "nonbasic land is a legal target"
    );
    assert!(
        !options.contains(&basic),
        "basic land is not a legal target"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![nonbasic],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, badlands()).is_some());
    assert!(on_battlefield(&engine, p1, badlands()).is_none());
    assert!(on_battlefield(&engine, p1, mountain()).is_some());
}
