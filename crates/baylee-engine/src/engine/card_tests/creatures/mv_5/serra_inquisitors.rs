//! Serra Inquisitors — `{4}{W}` 3/3 Creature — Human Cleric (Homelands)
//!
//! Whenever this creature blocks or becomes blocked by one or more black creatures, this creature
//! gets +2/+0 until end of turn.

#[allow(clippy::wildcard_imports)]
use super::*;

fn serra_inquisitors() -> CardIndex {
    card_index("3c6a32e8-0c2a-45cb-8b79-c7f022fe5129")
}

fn sengir_vampire() -> CardIndex {
    card_index("749141aa-f6c4-4ad8-b146-406e68ae9b0b")
}

#[test]
fn serra_inquisitors_gets_plus_two_plus_zero_when_blocked_by_black_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[serra_inquisitors()])
        .battlefield(1, &[sengir_vampire()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let inquisitors = on_battlefield(&engine, p0, serra_inquisitors()).unwrap();
    let vampire = on_battlefield(&engine, p1, sengir_vampire()).unwrap();
    assert_eq!(pt(&engine, inquisitors), (3, 3));

    // Attack with 3/3 Serra Inquisitors; p1 blocks with 4/4 black Sengir Vampire.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(inquisitors, Defender::Player(p1))],
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
                blockers: vec![(vampire, inquisitors)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    // Serra Inquisitors became 5/3 and traded with the 4/4 Sengir Vampire!
    assert_eq!(
        engine.state().object(vampire).unwrap().zone,
        Zone::Graveyard,
        "5/3 Serra Inquisitors killed the 4/4 black Sengir Vampire"
    );
}
