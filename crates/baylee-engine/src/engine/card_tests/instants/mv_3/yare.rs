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
    let menu = aim_at(&mut engine, p1, blocker);
    assert!(
        [a1, a2, a3].iter().all(|attacker| !menu.contains(attacker)),
        "\"creature defending player controls\": the attackers are no target"
    );
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

/// "Up to two additional creatures": three attackers can be blocked by the one
/// creature, a fourth cannot (the declaration is refused and nothing changes).
#[test]
fn yare_lets_a_creature_block_three_attackers_but_not_four() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                llanowar_elves(),
                savannah_lions(),
                mons_s_goblin_raiders(),
                grizzly_bears(),
            ],
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
    let a4 = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let blocker = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: [a1, a2, a3, a4]
                    .map(|attacker| (attacker, Defender::Player(p1)))
                    .to_vec(),
            },
        )
        .unwrap();
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, yare());
    aim_at(&mut engine, p1, blocker);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(blocker, a1), (blocker, a2), (blocker, a3), (blocker, a4)],
                },
            )
            .is_err(),
        "one creature blocking four attackers is one too many"
    );
    assert!(
        matches!(engine.pending(), Pending::ChooseBlockers { .. }),
        "the refused declaration changed nothing"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, a1), (blocker, a2), (blocker, a3)],
            },
        )
        .expect("the creature and two additional ones make three");
}

/// "This turn" and "until end of turn": the Bears that blocked a lone attacker
/// as a 5/2 is a 2/2 again on the opponent's own turn, the +3/+0 gone.
#[test]
fn yare_pump_ends_with_the_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[llanowar_elves()])
        .battlefield(1, &[grizzly_bears(), plains(), plains(), plains()])
        .hand(1, &[yare()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    let elf = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    let blocker = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elf, Defender::Player(p1))],
            },
        )
        .unwrap();
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, yare());
    aim_at(&mut engine, p1, blocker);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert_eq!(pt(&engine, blocker), (5, 2));
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, elf)],
            },
        )
        .unwrap();
    reach_their_main_phase(&mut engine, p1);

    assert!(in_graveyard(&engine, p0, llanowar_elves()).is_some());
    assert_eq!(
        pt(&engine, blocker),
        (2, 2),
        "the +3/+0 lasted until end of turn only"
    );
}
