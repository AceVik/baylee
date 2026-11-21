//! `cards/creatures/mv_4/norwood_riders.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn norwood_riders() -> CardIndex {
    card_index("1816b87c-73ba-4bfc-a51e-c982381c067d")
}

/// Norwood Riders cannot be blocked by two creatures at once, while a single
/// blocker may legally block it.
#[test]
fn norwood_riders_forbids_double_blocks_and_allows_a_single_blocker() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .battlefield(1, &[grizzly_bears(), gray_ogre()])
        .hand(0, &[norwood_riders()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, norwood_riders());
    pass_until(&mut engine, stack_is_empty);
    let riders = on_battlefield(&engine, p0, norwood_riders()).expect("resolved");
    assert_eq!(pt(&engine, riders), (3, 3));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    let _ = attack_and_collect_blocks(&mut engine, riders, p1);

    let Pending::ChooseBlockers { bounds, .. } = engine.pending() else {
        unreachable!("stopped on ChooseBlockers")
    };
    let bound = bounds
        .iter()
        .find(|b| b.attacker == riders)
        .expect("Norwood Riders bound");
    assert_eq!(bound.max_blockers, 1);

    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(bears, riders), (ogre, riders)],
                },
            )
            .is_err(),
        "two blockers must be rejected"
    );

    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bears, riders)],
            },
        )
        .expect("one blocker is legal");
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert!(in_graveyard(&engine, p1, grizzly_bears()).is_some());
    assert!(on_battlefield(&engine, p0, norwood_riders()).is_some());
}
