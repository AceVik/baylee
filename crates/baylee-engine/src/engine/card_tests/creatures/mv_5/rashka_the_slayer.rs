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

/// High Ground allows two blocks; "one or more" still creates just one pump.
#[test]
fn rashka_pumps_once_when_blocking_two_black_creatures() {
    use crate::engine::card_tests::legends_kit as lk;
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[index::SCATHE_ZOMBIES, index::SCATHE_ZOMBIES],
        &[rashka_the_slayer(), index::HIGH_GROUND],
        &[],
        &[],
    );
    let attackers: Vec<_> = e
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            e.state().object(*id).is_some_and(|o| {
                o.controller == p0 && o.card.is_some_and(|c| c.index == index::SCATHE_ZOMBIES)
            })
        })
        .collect();
    assert_eq!(attackers.len(), 2);
    let rashka = on_battlefield(&e, p1, rashka_the_slayer()).unwrap();
    let offers = lk::attack(&mut e, &attackers);
    assert!(
        attackers
            .iter()
            .all(|attacker| lk::may_block(&offers, rashka, *attacker))
    );
    lk::block(&mut e, &[(rashka, attackers[0]), (rashka, attackers[1])]);
    lk::settle(&mut e);
    assert_eq!(
        pt(&e, rashka),
        (4, 5),
        "one +1/+2 pump, rather than one per blocked attacker"
    );
}

/// The grouped block-or-become-blocked vocabulary must keep the printed direction.
#[test]
fn attacking_rashka_does_not_pump_when_a_black_creature_blocks_it() {
    use crate::engine::card_tests::legends_kit as lk;
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[rashka_the_slayer()],
        &[index::SCATHE_ZOMBIES],
        &[],
        &[],
    );
    let rashka = on_battlefield(&e, p0, rashka_the_slayer()).unwrap();
    let blocker = on_battlefield(&e, p1, index::SCATHE_ZOMBIES).unwrap();
    lk::attack(&mut e, &[rashka]);
    lk::block(&mut e, &[(blocker, rashka)]);
    lk::settle(&mut e);
    assert_eq!(pt(&e, rashka), (3, 3), "Rashka did not block a creature");
}
