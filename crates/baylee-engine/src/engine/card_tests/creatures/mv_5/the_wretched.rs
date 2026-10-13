//! Printed card behavior through the real engine.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn wretched_takes_only_its_surviving_blockers_and_releases_them_on_departure() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::THE_WRETCHED])
        .battlefield(1, &[giant_spider(), grizzly_bears()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let demon = on_battlefield(&e, p0, index::THE_WRETCHED).unwrap();
    let spider = on_battlefield(&e, p1, giant_spider()).unwrap();
    let bear = on_battlefield(&e, p1, grizzly_bears()).unwrap();
    attack_and_collect_blocks(&mut e, demon, p1);
    e.apply(
        p1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(spider, demon)],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(e.state().object(spider).unwrap().controller, p0);
    assert_eq!(e.state().object(bear).unwrap().controller, p1);
    kill(&mut e, demon);
    assert_eq!(e.state().object(spider).unwrap().controller, p1);
}

/// The Wretched attacks into `blockers` (declared in that order) with a
/// non-blocking Llanowar Elves held back on the same side, and the game is
/// walked to the second main phase, past "at end of combat".
fn wretched_after_combat_against(
    blockers: &[CardIndex],
    damaged: CardIndex,
) -> (Engine<RegistryLookup>, ObjectId, Vec<ObjectId>, ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut board = blockers.to_vec();
    board.push(llanowar_elves());
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::THE_WRETCHED])
        .battlefield(1, &board)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let demon = on_battlefield(&e, p0, index::THE_WRETCHED).unwrap();
    let blocking: Vec<_> = blockers
        .iter()
        .map(|b| on_battlefield(&e, p1, *b).unwrap())
        .collect();
    let bystander = on_battlefield(&e, p1, llanowar_elves()).unwrap();
    attack_and_collect_blocks(&mut e, demon, p1);
    e.apply(
        p1,
        PlayerAction::DeclareBlockers {
            blockers: blocking.iter().map(|b| (*b, demon)).collect(),
        },
    )
    .unwrap();
    // The Demon divides its 2 damage among its blockers: all of it to `damaged`.
    let damaged = on_battlefield(&e, p1, damaged).unwrap();
    for _ in 0..30 {
        if e.state().turn.phase == Phase::SecondMain {
            break;
        }
        if let Pending::ChooseNumber {
            player,
            min,
            max,
            reason: crate::choice::NumberPrompt::CombatDamage { recipient, .. },
        } = e.pending().clone()
        {
            let share = if recipient == damaged { max } else { min };
            e.apply(player, PlayerAction::ChooseNumber(share)).unwrap();
        } else {
            pass_until(&mut e, |e| {
                e.state().turn.phase == Phase::SecondMain
                    || matches!(e.pending(), Pending::ChooseNumber { .. })
            });
        }
    }
    assert_eq!(e.state().turn.phase, Phase::SecondMain);
    (e, demon, blocking, bystander)
}

#[test]
fn wretched_takes_every_blocker_and_only_the_blockers() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (e, _demon, blocking, bystander) =
        wretched_after_combat_against(&[giant_spider(), grizzly_bears()], giant_spider());
    for blocker in &blocking {
        assert_eq!(
            e.state().object(*blocker).unwrap().controller,
            p0,
            "all creatures blocking this creature change hands, not just the first"
        );
    }
    assert_eq!(
        e.state().object(bystander).unwrap().controller,
        p1,
        "a creature that did not block stays with its owner"
    );
}

#[test]
fn wretched_cannot_take_a_blocker_that_died_in_combat() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    // The Demon puts its whole 2 power on the Bears, and they die.
    let (e, _demon, blocking, bystander) =
        wretched_after_combat_against(&[grizzly_bears(), giant_spider()], grizzly_bears());
    let (bear, spider) = (blocking[0], blocking[1]);
    assert!(
        in_graveyard(&e, p1, grizzly_bears()).is_some(),
        "the 2/2 blocker died in combat damage and is in its owner's graveyard"
    );
    assert!(
        e.state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .all(|id| *id != bear),
        "the dead blocker is not on the battlefield for the Demon to take"
    );
    assert_eq!(
        e.state().object(spider).unwrap().controller,
        p0,
        "the surviving blocker is taken"
    );
    assert_eq!(e.state().object(bystander).unwrap().controller, p1);
}
