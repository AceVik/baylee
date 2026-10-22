//! `cards/enchantments/auras/mv_3/bestial_fury.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn bestial_fury() -> CardIndex {
    card_index("dfc08963-132f-4ebf-9384-15d17d1098f7")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// `Bestial Fury` ({2}{R}, Enchantment — Aura, `Coverage::Partial`) attaches to
/// target creature, draws a card at the beginning of the next turn's upkeep
/// when it enters, and gives enchanted creature +4/+0 and trample until end of
/// turn whenever it becomes blocked.
#[test]
fn bestial_fury_pumps_blocked_enchanted_creature_and_draws_next_upkeep() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), hill_giant()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[bestial_fury()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let giant = on_battlefield(&engine, p0, hill_giant()).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, bestial_fury());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![giant],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let fury = on_battlefield(&engine, p0, bestial_fury()).expect("Bestial Fury enters");
    assert_eq!(
        engine.state().object(fury).unwrap().attached_to,
        Some(giant)
    );

    // Hill Giant attacks and becomes blocked by Grizzly Bears -> gets +4/+0 and trample (7/3).
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.step == Step::DeclareAttackers
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(giant, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::DeclareBlockers
    });
    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bears, giant)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, giant), (7, 3));
    assert!(keywords(&engine, giant).contains(KeywordSet::TRAMPLE));

    // At the beginning of the next turn's upkeep (P1's upkeep), P0 draws a card.
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.step == Step::Upkeep
    });
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );
}
