#[allow(clippy::wildcard_imports)]
use super::*;

fn outmaneuver() -> CardIndex {
    card_index("42b44108-4a03-4333-bf5a-e42493f4d5e4")
}

#[test]
fn outmaneuver_targets_blocked_attacking_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[outmaneuver()])
        .battlefield(0, &[mountain(), mountain(), grizzly_bears()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    let elves = on_battlefield(&engine, p1, llanowar_elves()).expect("seated");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bear, baylee_core::ids::Defender::Player(p1))],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(elves, bear)],
            },
        )
        .unwrap();

    cast_from_hand(&mut engine, p0, outmaneuver());
    if matches!(engine.pending(), Pending::ChooseNumber { .. }) {
        engine.apply(p0, PlayerAction::ChooseNumber(1)).unwrap();
    }
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        aim_at(&mut engine, p0, bear);
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, outmaneuver()).is_some());
}
