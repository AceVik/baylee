//! `cards/creatures/mv_4/razorclaw_bear.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn razorclaw_bear() -> CardIndex {
    card_index("3ee07899-e6d6-4dcf-8596-4d2420690d7c")
}

/// "Whenever this creature becomes blocked" triggers once even against two
/// blockers (CR 509.3c), giving +2/+2 until end of turn rather than +4/+4.
#[test]
fn razorclaw_bear_gets_plus_two_plus_two_once_when_blocked_by_multiple_creatures() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .battlefield(1, &[grizzly_bears(), gray_ogre()])
        .hand(0, &[razorclaw_bear()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, razorclaw_bear());
    pass_until(&mut engine, stack_is_empty);
    let bear = on_battlefield(&engine, p0, razorclaw_bear()).expect("resolved");
    assert_eq!(pt(&engine, bear), (3, 3));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let b1 = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let b2 = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    let _ = attack_and_collect_blocks(&mut engine, bear, p1);
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(b1, bear), (b2, bear)],
            },
        )
        .unwrap();

    // Let the BecomesBlocked trigger resolve while still in DeclareBlockers.
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, bear),
        (5, 5),
        "becomes blocked triggers once (+2/+2), not per blocker"
    );

    // 5/5 assigns 2 lethal damage to the first 2/2 blocker and 3 to the second,
    // survives 4 total combat damage, kills both blockers, and resets next turn.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    assert!(on_battlefield(&engine, p0, razorclaw_bear()).is_some());
    assert!(in_graveyard(&engine, p1, grizzly_bears()).is_some());
    assert!(in_graveyard(&engine, p1, gray_ogre()).is_some());
    assert_eq!(pt(&engine, bear), (3, 3), "+2/+2 expires at end of turn");
}

/// Unblocked attack does not trigger `BecomesBlocked` and deals 3 damage.
#[test]
fn razorclaw_bear_stays_three_three_when_unblocked() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[razorclaw_bear()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, razorclaw_bear()).unwrap();
    let _ = attack_and_collect_blocks(&mut engine, bear, p1);
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(pt(&engine, bear), (3, 3));
    assert_eq!(engine.state().players[1].life, 17);
}
