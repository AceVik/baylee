//! `cards/creatures/mv_8/avatar_of_will.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn avatar_of_will() -> CardIndex {
    card_index("13aa88a0-4b47-4012-ba50-002e51e637cf")
}

#[test]
fn avatar_of_will_casts_and_flies_over_ground_blocker() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[avatar_of_will()])
        .battlefield(0, &[island(); 8])
        .battlefield(1, &[index::GRIZZLY_BEARS, index::AIR_ELEMENTAL])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, avatar_of_will());
    pass_until(&mut engine, stack_is_empty);
    let avatar = on_battlefield(&engine, p0, avatar_of_will()).unwrap();
    assert_eq!(pt(&engine, avatar), (5, 6));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).unwrap();
    let air = on_battlefield(&engine, p1, index::AIR_ELEMENTAL).unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(avatar, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(bears, avatar)],
                },
            )
            .is_err()
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(air, avatar)],
            },
        )
        .unwrap();
    reach_their_main_phase(&mut engine, p1);

    assert!(in_graveyard(&engine, p1, index::AIR_ELEMENTAL).is_some());
    assert!(on_battlefield(&engine, p0, avatar_of_will()).is_some());
}
