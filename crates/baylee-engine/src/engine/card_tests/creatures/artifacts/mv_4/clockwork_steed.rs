//! Clockwork Steed — `{4}` 0/3 Artifact Creature — Horse (Homelands)
//!
//! Clockwork Steed enters with four +1/+0 counters on it.
//! Clockwork Steed can't be blocked by artifact creatures.
//! At end of combat, if Clockwork Steed attacked or blocked this combat, remove
//! a +1/+0 counter from it.
//! `{X}`, `{T}`: Put up to X +1/+0 counters on Clockwork Steed. This ability
//! can't cause the total number of +1/+0 counters on Clockwork Steed to be
//! greater than four. Activate only during your upkeep.

#[allow(clippy::wildcard_imports)]
use super::*;

fn clockwork_steed() -> CardIndex {
    card_index("1d6a1dab-8724-4b07-b109-c6ba2a7ea1ad")
}

fn obsianus_golem() -> CardIndex {
    card_index("ac41171e-c454-49e9-9004-c082ae099630")
}

#[test]
fn clockwork_steed_cant_be_blocked_by_artifact_creatures_loses_counter_after_combat_and_refills_in_upkeep()
 {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[clockwork_steed(), forest(), forest()])
        .battlefield(1, &[obsianus_golem(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let steed = on_battlefield(&engine, p0, clockwork_steed()).unwrap();
    let golem = on_battlefield(&engine, p1, obsianus_golem()).unwrap();
    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    assert_eq!(plus_one_zero(&engine, steed), 4);
    assert_eq!(pt(&engine, steed), (4, 3));

    // Attack with Clockwork Steed: Obsianus Golem (artifact creature) cannot block it, Grizzly Bears can.
    let block_options = attack_and_collect_blocks(&mut engine, steed, p1);
    assert!(
        !block_options
            .iter()
            .any(|o| o.blocker == golem && o.attackers.contains(&steed)),
        "artifact creature cannot block Clockwork Steed"
    );
    assert!(
        block_options
            .iter()
            .any(|o| o.blocker == bears && o.attackers.contains(&steed)),
        "nonartifact creature can block Clockwork Steed"
    );

    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(engine.state().players[1].life, 16);
    assert_eq!(
        plus_one_zero(&engine, steed),
        3,
        "lost 1 +1/+0 counter at end of combat"
    );
    assert_eq!(pt(&engine, steed), (3, 3));

    // In p0's next upkeep, refill with X=2 (capped at 4 total counters).
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0
            && e.state().turn.step == Step::Upkeep
            && e.state().turn.number > 1
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, clockwork_steed(), 2);
    engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    // X = 2 but only one counter of room: "can't cause the total ... to be
    // greater than four" caps the number at 1.
    let Pending::ChooseNumber { min, max, .. } = engine.pending().clone() else {
        panic!("expected the number of counters");
    };
    assert_eq!(
        (min, max),
        (0, 1),
        "three counters plus at most one is four"
    );
    engine.apply(p0, PlayerAction::ChooseNumber(1)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(plus_one_zero(&engine, steed), 4);
    assert_eq!(pt(&engine, steed), (4, 3));

    // No attack on this turn (the Steed is tapped): the end-of-combat trigger
    // removes nothing, because it did not attack or block this combat.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0
            && e.state().turn.phase == Phase::SecondMain
            && stack_is_empty(e)
    });
    assert_eq!(
        plus_one_zero(&engine, steed),
        4,
        "a combat the Steed did not attack or block in costs it no counter"
    );
}
