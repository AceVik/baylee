//! `cards/creatures/mv_3/task_force.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

// oracle_id = "b80415cd-5887-4146-852e-73119fcd3ec6"
fn task_force() -> CardIndex {
    card_index("b80415cd-5887-4146-852e-73119fcd3ec6")
}

/// `Task Force` (`Coverage::Implemented`):
/// "Whenever this creature becomes the target of a spell or ability, it gets
/// +0/+3 until end of turn."
#[test]
fn task_force_gets_plus_zero_plus_three_when_targeted() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(9704, forest())
        .battlefield(0, &[task_force(), forest()])
        .hand(0, &[giant_growth()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let force = on_battlefield(&engine, p0, task_force()).expect("Task Force");
    assert_eq!(pt(&engine, force), (1, 3));

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, giant_growth());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![force],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Giant Growth gives +3/+3 and Task Force's trigger gives +0/+3 -> 4/9.
    assert_eq!(
        pt(&engine, force),
        (4, 9),
        "Task Force gets +0/+3 from its trigger plus +3/+3 from Giant Growth"
    );
}
