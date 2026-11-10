//! `cards/instants/mv_1/righteousness.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Righteousness: "Target creature blocking one or more creatures gets
/// +7/+7 until end of turn." A 1/1 blocking a 4/6 becomes an 8/8: it takes
/// the hit and lives, and deals enough back to kill the attacker — neither
/// of which a 1/1 does on its own.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played through real combat
fn righteousness_pumps_the_blocking_creature_seven_seven() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[quiet_creature(), plains()])
        .hand(0, &[righteousness()])
        .battlefield(1, &[obsianus_golem()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    let golem = on_battlefield(&engine, p1, obsianus_golem()).expect("the Golem is seated");
    let blocker = on_battlefield(&engine, p0, quiet_creature()).expect("the Elf is seated");
    assert_eq!(pt(&engine, blocker), (1, 1), "before the pump");
    let blocks = attack_and_collect_blocks(&mut engine, golem, p0);
    assert!(
        blocks
            .iter()
            .any(|b| b.blocker == blocker && b.attackers.contains(&golem)),
        "a 1/1 may block a 4/6: {blocks:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, golem)],
            },
        )
        .expect("the Elf blocks the Golem");

    pass_until_priority(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, righteousness());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&blocker),
        "the blocking creature is a legal target: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![blocker],
                players: vec![],
            },
        )
        .expect("the blocker is legal");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, blocker), (8, 8), "\"+7/+7\"");

    let before = life_of(&engine, p0);
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain) && e.state().turn.active == p1
    });
    assert!(
        on_battlefield(&engine, p0, quiet_creature()).is_some(),
        "the 8/8 survives 4 damage"
    );
    assert!(
        in_graveyard(&engine, p1, obsianus_golem()).is_some(),
        "and deals 8 back to a 6-toughness attacker"
    );
    assert_eq!(life_of(&engine, p0), before, "blocked, and no trample");
}

/// Righteousness: "Target blocking creature". Only a creature that is
/// blocking is a target: the attacking Golem and a second Ogre that did not
/// block are not offered, and naming the Ogre is refused. The +7/+7 lasts
/// the rest of the turn and is gone when the next turn begins.
#[test]
#[allow(clippy::too_many_lines)] // one combat, the menu read in the middle
fn righteousness_targets_only_a_blocking_creature_and_ends_with_the_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[quiet_creature(), gray_ogre(), plains()])
        .hand(0, &[righteousness()])
        .battlefield(1, &[obsianus_golem()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    let golem = on_battlefield(&engine, p1, obsianus_golem()).expect("the Golem");
    let blocker = on_battlefield(&engine, p0, quiet_creature()).expect("the Elf");
    let idler = on_battlefield(&engine, p0, gray_ogre()).expect("the Ogre");
    attack_and_collect_blocks(&mut engine, golem, p0);
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, golem)],
            },
        )
        .expect("the Elf blocks the Golem");
    pass_until_priority(&mut engine, p0);

    let plains_id = on_battlefield(&engine, p0, plains()).expect("the Plains");
    tap_mana_where(&mut engine, p0, |id| id == plains_id);
    cast_with_floating(&mut engine, p0, righteousness());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert!(options.contains(&blocker), "{options:?}");
    assert!(
        !options.contains(&idler),
        "a creature that is not blocking is not a target: {options:?}"
    );
    assert!(
        !options.contains(&golem),
        "nor is the attacker: {options:?}"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![idler],
                    players: vec![],
                },
            )
            .is_err(),
        "naming the idle Ogre anyway is refused"
    );
    aim_at(&mut engine, p0, blocker);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, blocker), (8, 8), "\"+7/+7\"");

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain) && e.state().turn.active == p1
    });
    assert_eq!(
        pt(&engine, blocker),
        (8, 8),
        "still pumped after combat, in the same turn"
    );
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, blocker),
        (1, 1),
        "the pump is gone after the cleanup step"
    );
}
