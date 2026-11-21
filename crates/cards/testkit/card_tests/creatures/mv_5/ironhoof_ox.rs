//! `cards/creatures/mv_5/ironhoof_ox.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn ironhoof_ox() -> CardIndex {
    card_index("1871af55-f7cd-4b8e-affb-5d010df811b1")
}

/// Ironhoof Ox cannot be blocked by more than one creature, while a single
/// blocker is permitted.
#[test]
fn ironhoof_ox_forbids_double_blocks_and_allows_a_single_blocker() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 5])
        .battlefield(1, &[grizzly_bears(), gray_ogre()])
        .hand(0, &[ironhoof_ox()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, ironhoof_ox());
    pass_until(&mut engine, stack_is_empty);
    let ox = on_battlefield(&engine, p0, ironhoof_ox()).expect("resolved");
    assert_eq!(pt(&engine, ox), (4, 4));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    let _ = attack_and_collect_blocks(&mut engine, ox, p1);

    let Pending::ChooseBlockers { bounds, .. } = engine.pending() else {
        unreachable!("stopped on ChooseBlockers")
    };
    let bound = bounds
        .iter()
        .find(|b| b.attacker == ox)
        .expect("Ironhoof Ox bound");
    assert_eq!(bound.max_blockers, 1);

    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(bears, ox), (ogre, ox)],
                },
            )
            .is_err(),
        "two blockers must be rejected"
    );

    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(ogre, ox)],
            },
        )
        .expect("one blocker is legal");
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert!(in_graveyard(&engine, p1, gray_ogre()).is_some());
    assert!(on_battlefield(&engine, p0, ironhoof_ox()).is_some());
}
