//! Rashka the Slayer — `{3}{W}{W}` 3/3 Legendary Creature — Human Archer (Homelands)
//!
//! Reach
//! Whenever Rashka blocks one or more black creatures, Rashka gets +1/+2 until
//! end of turn.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn rashka_the_slayer() -> CardIndex {
    card_index("8e1e477a-192a-4e5c-a18b-7e25d9bd47f3")
}

#[test]
fn rashka_the_slayer_has_reach_and_pumps_when_blocking_black_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[index::SENGIR_VAMPIRE])
        .battlefield(1, &[rashka_the_slayer()])
        .start();
    keep_mulligans(&mut engine);

    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let vampire = on_battlefield(&engine, p0, index::SENGIR_VAMPIRE).unwrap();
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
