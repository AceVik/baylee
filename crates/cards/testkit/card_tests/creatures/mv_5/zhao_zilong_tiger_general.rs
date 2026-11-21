//! `cards/creatures/mv_5/zhao_zilong_tiger_general.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn zhao_zilong_tiger_general() -> CardIndex {
    card_index("22aa8e7f-b30d-49e2-b99e-9232e1a3bcf3")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// Zhao Zilong, Tiger General (22aa8e7f-b30d-49e2-b99e-9232e1a3bcf3) — `{3}{W}{W}` 3/3
/// Legendary Creature — Human Soldier Warrior:
/// - Horsemanship
/// - Whenever this creature blocks, it gets +1/+1 until end of turn.
#[test]
fn zhao_zilong_tiger_general_pumps_when_blocking_but_not_when_attacking() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains(), plains()])
        .battlefield(1, &[hill_giant()])
        .hand(0, &[zhao_zilong_tiger_general()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, zhao_zilong_tiger_general());
    pass_until(&mut engine, stack_is_empty);

    let zz = on_battlefield(&engine, P0, zhao_zilong_tiger_general()).expect("Zhao Zilong entered");
    let giant = on_battlefield(&engine, P1, hill_giant()).unwrap();
    assert_eq!(pt(&engine, zz), (3, 3));
    assert!(keywords(&engine, zz).contains(KeywordSet::HORSEMANSHIP));

    // P1's turn: 3/3 Hill Giant attacks; Zhao Zilong blocks, becomes 4/4, kills Giant, survives.
    reach_their_main_phase(&mut engine, P1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(giant, Defender::Player(P0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(zz, giant)],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(pt(&engine, zz), (4, 4));
    assert!(in_graveyard(&engine, P1, hill_giant()).is_some());
    assert!(on_battlefield(&engine, P0, zhao_zilong_tiger_general()).is_some());

    // P0's next turn: bonus expired; attacking does not trigger `Blocks`, deals 3 damage.
    reach_their_main_phase(&mut engine, P0);
    assert_eq!(pt(&engine, zz), (3, 3));
    let _ = attack_and_collect_blocks(&mut engine, zz, P1);
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(pt(&engine, zz), (3, 3));
    assert_eq!(engine.state().players[1].life, 17);
}
