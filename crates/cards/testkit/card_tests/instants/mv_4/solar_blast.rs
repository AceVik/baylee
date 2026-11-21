//! `cards/instants/mv_4/solar_blast.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
#[allow(clippy::wildcard_imports)]
use baylee_card_testkit::card_tests::legends_kit::*;

fn solar_blast() -> CardIndex {
    card_index("ece3e379-c586-48bd-8742-472b5deccc35")
}

/// Cast normally, Solar Blast deals 3 damage to any target.
#[test]
fn solar_blast_cast_deals_three_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .hand(0, &[solar_blast()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, solar_blast());
    aim(&mut engine, &[], &[p1]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life(&engine, p1), 17);
}

/// Cycling Solar Blast for `{1}{R}{R}` optionally deals 1 damage to any target
/// (creature or player) before drawing, and may be declined while still drawing.
#[test]
fn solar_blast_cycling_deals_one_damage_or_declines() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 9])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[solar_blast(), solar_blast(), solar_blast()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("P1 Elf");
    let lands = all_on_battlefield(&engine, p0, mountain());
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // 1. Cycle and accept targeting 1/1 Elf -> kills it before cycling draw.
    tap_mana_where(&mut engine, p0, |id| lands[..3].contains(&id));
    activate(&mut engine, p0, solar_blast(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim(&mut engine, &[elf], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    yes_no(&mut engine, true);
    assert_eq!(zone_of(&engine, elf), Zone::Graveyard);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1,
        "damage trigger resolved before cycling draw"
    );
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );

    // 2. Cycle and accept targeting opponent P1 -> 1 damage (20 -> 19).
    tap_mana_where(&mut engine, p0, |id| lands[3..6].contains(&id));
    activate(&mut engine, p0, solar_blast(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim(&mut engine, &[], &[p1]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    yes_no(&mut engine, true);
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(life(&engine, p1), 19);

    // 3. Cycle and decline targeting opponent P1 -> 0 damage, still draws.
    tap_mana_where(&mut engine, p0, |id| lands[6..9].contains(&id));
    activate(&mut engine, p0, solar_blast(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim(&mut engine, &[], &[p1]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    yes_no(&mut engine, false);
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(life(&engine, p1), 19);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );
}
