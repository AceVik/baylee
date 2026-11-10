//! Folk of An-Havva — `{G}` 1/1 Creature — Human (Homelands)
//!
//! Whenever Folk of An-Havva blocks, it gets +2/+0 until end of turn.

#[allow(clippy::wildcard_imports)]
use super::*;

fn folk_of_an_havva() -> CardIndex {
    card_index("86002526-ed3d-47e5-9f2a-55c4fa220ce8")
}

fn wall_of_stone() -> CardIndex {
    card_index("cd4cadb4-3156-49bd-b36e-12ba5c85938b")
}

#[test]
fn folk_of_an_havva_gets_plus_two_plus_zero_when_blocking_and_not_when_attacking() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[folk_of_an_havva()])
        .battlefield(1, &[grizzly_bears(), wall_of_stone()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let folk = on_battlefield(&engine, p0, folk_of_an_havva()).unwrap();
    let wall = on_battlefield(&engine, p1, wall_of_stone()).unwrap();
    let p1_bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    assert_eq!(pt(&engine, folk), (1, 1));

    // When Folk of An-Havva attacks and is blocked by 0/8 Wall of Stone, it stays 1/1.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(folk, Defender::Player(p1))],
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
                blockers: vec![(wall, folk)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(pt(&engine, folk), (1, 1));
    assert_eq!(engine.state().object(wall).unwrap().damage, 1);

    // Untap Folk for p1's attack test (or advance through p1's turn after p0's next untap).
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain
    });
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });

    // p1 attacks with 2/2 Grizzly Bears; p0 blocks with 1/1 Folk of An-Havva -> Folk becomes 3/1 and kills the 2/2!
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(p1_bears, Defender::Player(p0))],
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
                blockers: vec![(folk, p1_bears)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(
        engine.state().object(p1_bears).unwrap().zone,
        Zone::Graveyard,
        "3/1 blocking Folk of An-Havva deals 3 combat damage and kills the 2/2 Grizzly Bears"
    );
}
