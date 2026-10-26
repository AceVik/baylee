//! `cards/creatures/mv_8/avatar_of_hope.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn avatar_of_hope() -> CardIndex {
    card_index("e2f22cef-9494-4356-aa5c-619aecd7e74d")
}

#[test]
fn avatar_of_hope_casts_and_blocks_multiple_attackers() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[avatar_of_hope()])
        .battlefield(0, &[plains(); 8])
        .battlefield(
            1,
            &[index::GRIZZLY_BEARS, index::GRAY_OGRE, index::AIR_ELEMENTAL],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, avatar_of_hope());
    pass_until(&mut engine, stack_is_empty);
    let avatar = on_battlefield(&engine, p0, avatar_of_hope()).unwrap();
    assert_eq!(pt(&engine, avatar), (4, 9));

    let bears = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).unwrap();
    let ogre = on_battlefield(&engine, p1, index::GRAY_OGRE).unwrap();
    let air = on_battlefield(&engine, p1, index::AIR_ELEMENTAL).unwrap();

    reach_their_main_phase(&mut engine, p1);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (bears, Defender::Player(p0)),
                    (ogre, Defender::Player(p0)),
                    (air, Defender::Player(p0)),
                ],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(avatar, bears), (avatar, ogre), (avatar, air)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
    engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
    reach_their_main_phase(&mut engine, p0);

    assert!(on_battlefield(&engine, p0, avatar_of_hope()).is_some());
    assert!(in_graveyard(&engine, p1, index::GRIZZLY_BEARS).is_some());
    assert!(in_graveyard(&engine, p1, index::GRAY_OGRE).is_some());
    assert_eq!(engine.state().players[0].life, 20);
}
