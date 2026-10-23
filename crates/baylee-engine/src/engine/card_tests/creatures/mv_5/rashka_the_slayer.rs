//! Rashka the Slayer — `{3}{W}{W}` 3/3 Legendary Creature — Human Archer (Homelands)
//!
//! Reach
//! Whenever Rashka blocks one or more black creatures, Rashka gets +1/+2 until
//! end of turn.

#[allow(clippy::wildcard_imports)]
use super::*;

fn rashka_the_slayer() -> CardIndex {
    card_index("8e1e477a-192a-4e5c-a18b-7e25d9bd47f3")
}

#[test]
fn rashka_the_slayer_has_reach_and_pumps_when_blocking_black_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[card_index("749141aa-f6c4-4ad8-b146-406e68ae9b0b")])
        .battlefield(1, &[rashka_the_slayer()])
        .start();
    keep_mulligans(&mut engine);

    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let vampire = on_battlefield(
        &engine,
        p0,
        card_index("749141aa-f6c4-4ad8-b146-406e68ae9b0b"),
    )
    .unwrap();
    let rashka = on_battlefield(&engine, p1, rashka_the_slayer()).unwrap();

    // Sengir Vampire is a 4/4 black creature with flying; Rashka has reach so can block it!
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(vampire, Defender::Player(p1))],
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
                blockers: vec![(rashka, vampire)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, rashka), (4, 5));
}

/// "Black creatures" only: Rashka blocking a green creature stays a 3/3.
#[test]
fn rashka_the_slayer_does_not_pump_when_blocking_a_creature_that_is_not_black() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")])
        .battlefield(1, &[rashka_the_slayer()])
        .start();
    keep_mulligans(&mut engine);

    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let bears = on_battlefield(
        &engine,
        p0,
        card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0"),
    )
    .unwrap();
    let rashka = on_battlefield(&engine, p1, rashka_the_slayer()).unwrap();

    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(p1))],
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
                blockers: vec![(rashka, bears)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(
        pt(&engine, rashka),
        (3, 3),
        "a green attacker gives no pump"
    );
}
