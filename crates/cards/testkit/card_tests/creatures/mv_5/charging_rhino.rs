//! `cards/creatures/mv_5/charging_rhino.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn charging_rhino() -> CardIndex {
    card_index("26966ecb-15d3-47e5-ab63-e38510c87ecc")
}

/// Charging Rhino cannot be blocked by two creatures at once, while one
/// blocker is legal and dies to the 4/4 body.
#[test]
fn charging_rhino_forbids_double_blocks_and_allows_a_single_blocker() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 5])
        .battlefield(1, &[grizzly_bears(), gray_ogre()])
        .hand(0, &[charging_rhino()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, charging_rhino());
    pass_until(&mut engine, stack_is_empty);
    let rhino = on_battlefield(&engine, p0, charging_rhino()).expect("resolved");
    assert_eq!(pt(&engine, rhino), (4, 4));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    let _ = attack_and_collect_blocks(&mut engine, rhino, p1);

    let Pending::ChooseBlockers { bounds, .. } = engine.pending() else {
        unreachable!("stopped on ChooseBlockers")
    };
    let bound = bounds
        .iter()
        .find(|b| b.attacker == rhino)
        .expect("Charging Rhino bound");
    assert_eq!(bound.max_blockers, 1);

    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(bears, rhino), (ogre, rhino)],
                },
            )
            .is_err(),
        "two blockers must be rejected"
    );

    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bears, rhino)],
            },
        )
        .expect("one blocker is legal");
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert!(in_graveyard(&engine, p1, grizzly_bears()).is_some());
    assert!(on_battlefield(&engine, p0, charging_rhino()).is_some());
}
