//! `cards/creatures/mv_8/avatar_of_might.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn avatar_of_might() -> CardIndex {
    card_index("68d84ffa-5bd0-4ed9-a987-6cef13095438")
}

#[test]
fn avatar_of_might_casts_and_tramples_over_blocker() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[avatar_of_might()])
        .battlefield(0, &[forest(); 8])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, avatar_of_might());
    pass_until(&mut engine, stack_is_empty);
    let avatar = on_battlefield(&engine, p0, avatar_of_might()).unwrap();
    assert_eq!(pt(&engine, avatar), (8, 8));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).unwrap();
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
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bears, avatar)],
            },
        )
        .unwrap();
    reach_their_main_phase(&mut engine, p1);

    assert!(in_graveyard(&engine, p1, index::GRIZZLY_BEARS).is_some());
    assert_eq!(engine.state().players[1].life, 14);
}
