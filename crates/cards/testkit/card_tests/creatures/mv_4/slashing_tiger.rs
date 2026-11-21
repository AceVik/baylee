//! `cards/creatures/mv_4/slashing_tiger.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn slashing_tiger() -> CardIndex {
    card_index("c07a86de-ec29-478c-b6cb-057eb9d3e126")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Slashing Tiger (c07a86de-ec29-478c-b6cb-057eb9d3e126) — `{2}{G}{G}` 3/3
/// Creature — Cat:
/// "Whenever this creature becomes blocked, it gets +2/+2 until end of turn."
#[test]
fn slashing_tiger_stays_three_three_when_unblocked_and_pumps_once_when_blocked() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .battlefield(1, &[hill_giant()])
        .hand(0, &[slashing_tiger()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, slashing_tiger());
    pass_until(&mut engine, stack_is_empty);
    let tiger = on_battlefield(&engine, P0, slashing_tiger()).expect("Slashing Tiger entered");
    let giant = on_battlefield(&engine, P1, hill_giant()).unwrap();
    assert_eq!(pt(&engine, tiger), (3, 3));

    // Unblocked combat: no `BecomesBlocked` trigger, deals 3 damage.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(tiger, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(pt(&engine, tiger), (3, 3));
    assert_eq!(engine.state().players[1].life, 17);

    // Blocked by 3/3 Hill Giant: becomes 5/5 until end of turn, kills Hill Giant, survives.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(tiger, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(giant, tiger)],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(pt(&engine, tiger), (5, 5));
    assert!(in_graveyard(&engine, P1, hill_giant()).is_some());
    assert!(on_battlefield(&engine, P0, slashing_tiger()).is_some());

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, tiger), (3, 3), "pump ends at cleanup");
}

#[test]
fn slashing_tiger_double_blocked_triggers_becomes_blocked_only_once() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[slashing_tiger()])
        .battlefield(1, &[grizzly_bears(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);

    let tiger = on_battlefield(&engine, P0, slashing_tiger()).unwrap();
    let bears: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| engine.state().object(id).unwrap().controller == P1)
        .collect();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(tiger, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bears[0], tiger), (bears[1], tiger)],
            },
        )
        .unwrap();

    // Resolve the single `BecomesBlocked` trigger before combat damage step.
    pass_until(&mut engine, |e| {
        stack_is_empty(e) && e.state().turn.step == Step::DeclareBlockers
    });
    assert_eq!(
        pt(&engine, tiger),
        (5, 5),
        "CR 509.3c: `BecomesBlocked` triggers once regardless of blocker count"
    );
}
