//! Clockwork Swarm — `{4}` 0/3 Artifact Creature — Insect (Homelands)
//!
//! Clockwork Swarm enters with four +1/+0 counters on it.
//! Clockwork Swarm can't be blocked by Walls.
//! At end of combat, if Clockwork Swarm attacked or blocked this combat, remove
//! a +1/+0 counter from it.
//! `{X}`, `{T}`: Put up to X +1/+0 counters on Clockwork Swarm. This ability
//! can't cause the total number of +1/+0 counters on Clockwork Swarm to be
//! greater than four. Activate only during your upkeep.

#[allow(clippy::wildcard_imports)]
use super::*;

fn clockwork_swarm() -> CardIndex {
    card_index("a6091c3e-b227-4e84-b75d-69512a0bddfb")
}

fn wall_of_stone() -> CardIndex {
    card_index("cd4cadb4-3156-49bd-b36e-12ba5c85938b")
}

#[test]
fn clockwork_swarm_cant_be_blocked_by_walls_loses_counter_after_combat_and_refills_in_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[clockwork_swarm(), forest(), forest()])
        .battlefield(1, &[wall_of_stone(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let swarm = on_battlefield(&engine, p0, clockwork_swarm()).unwrap();
    let wall = on_battlefield(&engine, p1, wall_of_stone()).unwrap();
    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    assert_eq!(plus_one_zero(&engine, swarm), 4);
    assert_eq!(pt(&engine, swarm), (4, 3));

    // Attack with Clockwork Swarm: Wall of Stone cannot block it, Grizzly Bears can.
    let block_options = attack_and_collect_blocks(&mut engine, swarm, p1);
    assert!(
        !block_options
            .iter()
            .any(|o| o.blocker == wall && o.attackers.contains(&swarm)),
        "Wall cannot block Clockwork Swarm"
    );
    assert!(
        block_options
            .iter()
            .any(|o| o.blocker == bears && o.attackers.contains(&swarm)),
        "non-Wall creature can block Clockwork Swarm"
    );

    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(engine.state().players[1].life, 16);
    assert_eq!(
        plus_one_zero(&engine, swarm),
        3,
        "lost 1 +1/+0 counter at end of combat"
    );
    assert_eq!(pt(&engine, swarm), (3, 3));

    // In p0's next upkeep, refill with X=2 when only one counter fits.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0
            && e.state().turn.step == Step::Upkeep
            && e.state().turn.number > 1
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, clockwork_swarm(), 2);
    engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    let Pending::ChooseNumber { min, max, .. } = engine.pending().clone() else {
        panic!("expected the number of counters");
    };
    assert_eq!(
        (min, max),
        (0, 1),
        "X = 2 but three counters plus at most one is four"
    );
    engine.apply(p0, PlayerAction::ChooseNumber(1)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(plus_one_zero(&engine, swarm), 4);
    assert_eq!(pt(&engine, swarm), (4, 3));
}
