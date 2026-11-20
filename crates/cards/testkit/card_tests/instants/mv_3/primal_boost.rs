//! `cards/instants/mv_3/primal_boost.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
#[allow(clippy::wildcard_imports)]
use baylee_card_testkit::card_tests::legends_kit::*;

fn primal_boost() -> CardIndex {
    card_index("902e7d4b-7ab4-40b5-b3a5-c83ef604989a")
}

/// Cast normally, Primal Boost gives target creature +4/+4 until end of turn.
#[test]
fn primal_boost_cast_gives_plus_four_until_eot() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), llanowar_elves()])
        .hand(0, &[primal_boost()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("Elf seated");
    let lands = all_on_battlefield(&engine, p0, forest());
    tap_mana_where(&mut engine, p0, |id| lands.contains(&id));
    cast_with_floating(&mut engine, p0, primal_boost());
    aim(&mut engine, &[elf], &[]);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, elf), (5, 5));
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(pt(&engine, elf), (1, 1));
}

/// Cycling Primal Boost optionally gives target creature +1/+1 until end of
/// turn before drawing, and may be declined while still drawing a card.
#[test]
fn primal_boost_cycling_gives_optional_plus_one_until_eot() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[primal_boost(), primal_boost()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("Elf seated");
    let lands = all_on_battlefield(&engine, p0, forest());
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // 1. Cycle and accept +1/+1.
    tap_mana_where(&mut engine, p0, |id| lands[..3].contains(&id));
    activate(&mut engine, p0, primal_boost(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim(&mut engine, &[elf], &[]);
    pass_until(&mut engine, |e| matches!(e.pending(), Pending::YesNo { .. }));
    yes_no(&mut engine, true);
    assert_eq!(pt(&engine, elf), (2, 2));
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1,
        "pump resolved before cycling draw"
    );
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );

    // 2. Cycle and decline -> stays 2/2, still draws.
    tap_mana_where(&mut engine, p0, |id| lands[3..6].contains(&id));
    activate(&mut engine, p0, primal_boost(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim(&mut engine, &[elf], &[]);
    pass_until(&mut engine, |e| matches!(e.pending(), Pending::YesNo { .. }));
    yes_no(&mut engine, false);
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(pt(&engine, elf), (2, 2));
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(pt(&engine, elf), (1, 1));
}
