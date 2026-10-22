//! `cards/instants/mv_3/yare.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn yare() -> CardIndex {
    card_index("f0f24c6d-80fb-4e99-a74d-6ed9d349ed3d")
}

fn savannah_lions() -> CardIndex {
    baylee_core::generated::index::SAVANNAH_LIONS
}

fn mons_s_goblin_raiders() -> CardIndex {
    baylee_core::generated::index::MONS_S_GOBLIN_RAIDERS
}

/// Yare (`{2}{W}`, `Coverage::Implemented`): Instant that gives target creature
/// defending player controls +3/+0 until end of turn and lets it block up to
/// two additional creatures this turn.
#[test]
fn yare_pumps_defending_creature_and_lets_it_block_three_attackers() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[llanowar_elves(), savannah_lions(), mons_s_goblin_raiders()],
        )
        .battlefield(1, &[grizzly_bears(), plains(), plains(), plains()])
        .hand(1, &[yare()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let a1 = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    let a2 = on_battlefield(&engine, p0, savannah_lions()).unwrap();
    let a3 = on_battlefield(&engine, p0, mons_s_goblin_raiders()).unwrap();
    let blocker = on_battlefield(&engine, p1, grizzly_bears()).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (a1, Defender::Player(p1)),
                    (a2, Defender::Player(p1)),
                    (a3, Defender::Player(p1)),
                ],
            },
        )
        .unwrap();

    // p0 passes priority in DeclareAttackers; p1 casts Yare on Grizzly Bears.
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, yare());
    aim_at(&mut engine, p1, blocker);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });

    assert_eq!(pt(&engine, blocker), (5, 2));

    // Grizzly Bears blocks all three attacking creatures.
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, a1), (blocker, a2), (blocker, a3)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    engine.apply(p1, PlayerAction::ChooseNumber(2)).unwrap();
    engine.apply(p1, PlayerAction::ChooseNumber(2)).unwrap();
    reach_their_main_phase(&mut engine, p1);

    assert_eq!(engine.state().players[1].life, 20);
    assert!(in_graveyard(&engine, p0, llanowar_elves()).is_some());
    assert!(in_graveyard(&engine, p0, savannah_lions()).is_some());
    assert!(in_graveyard(&engine, p0, mons_s_goblin_raiders()).is_some());
}
