//! Greater Werewolf — `{4}{B}` 2/4 Creature — Werewolf (Homelands)
//!
//! At end of combat, put a -0/-2 counter on each creature blocking or blocked
//! by this creature.

#[allow(clippy::wildcard_imports)]
use super::*;

fn greater_werewolf() -> CardIndex {
    card_index("02847b0d-03a1-4e30-902e-60bdf6952c58")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

#[test]
fn greater_werewolf_puts_minus_zero_minus_two_counter_on_creatures_blocking_or_blocked_by_it() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[greater_werewolf()])
        .battlefield(1, &[hill_giant(), bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let werewolf = on_battlefield(&engine, p0, greater_werewolf()).unwrap();
    let blocker = on_battlefield(&engine, p1, hill_giant()).unwrap();
    let bystander = on_battlefield(&engine, p1, bears()).unwrap();

    // Attack with 2/4 Greater Werewolf; p1 blocks with 3/3 Hill Giant.
    // Hill Giant takes 2 combat damage and gets a -0/-2 counter at end of combat -> becomes 3/1 with 2 damage -> dies!
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(werewolf, Defender::Player(p1))],
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
                blockers: vec![(blocker, werewolf)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(
        engine.state().object(blocker).unwrap().zone,
        Zone::Graveyard,
        "3/3 Hill Giant with 2 combat damage died when Greater Werewolf put a -0/-2 counter on it at end of combat"
    );
    assert_eq!(
        engine
            .state()
            .object(bystander)
            .unwrap()
            .counters
            .get(CounterKind::Minus {
                power: 0,
                toughness: 2,
            }),
        0,
        "uninvolved creature received no -0/-2 counter"
    );
}

/// "Blocking or blocked by": the Werewolf on defence puts the counter on the
/// attacker it blocked, and not on the attacker it did not.
#[test]
fn greater_werewolf_puts_the_counter_on_the_attacker_it_blocks() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[hill_giant(), bears()])
        .battlefield(1, &[greater_werewolf()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let werewolf = on_battlefield(&engine, p1, greater_werewolf()).unwrap();
    let blocked = on_battlefield(&engine, p0, hill_giant()).unwrap();
    let unblocked = on_battlefield(&engine, p0, bears()).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (blocked, Defender::Player(p1)),
                    (unblocked, Defender::Player(p1)),
                ],
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
                blockers: vec![(werewolf, blocked)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    // The 2/4 Werewolf dealt 2 to the 3/3 Giant, which survives that damage
    // alone: it died to the -0/-2 counter.
    assert_eq!(
        engine.state().object(blocked).unwrap().zone,
        Zone::Graveyard,
        "the attacker the Werewolf blocked got a -0/-2 counter and died of its 2 damage"
    );
    assert_eq!(
        engine
            .state()
            .object(unblocked)
            .unwrap()
            .counters
            .get(CounterKind::Minus {
                power: 0,
                toughness: 2,
            }),
        0,
        "the unblocked attacker got no counter"
    );
}
