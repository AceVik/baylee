//! `cards/instants/mv_4/death_pulse.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
#[allow(clippy::wildcard_imports)]
use baylee_card_testkit::card_tests::legends_kit::*;

fn death_pulse() -> CardIndex {
    card_index("2227bab2-36a0-47f1-a797-7b82323eca9d")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Cast normally, Death Pulse gives target creature -4/-4 until end of turn,
/// killing a 4/4 Serra Angel via state-based actions.
#[test]
fn death_pulse_cast_gives_minus_four_killing_4_4() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .battlefield(1, &[serra_angel()])
        .hand(0, &[death_pulse()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let angel = on_battlefield(&engine, p1, serra_angel()).expect("P1 Angel");
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, death_pulse());
    aim(&mut engine, &[angel], &[]);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(zone_of(&engine, angel), Zone::Graveyard);
}

/// Cycling Death Pulse for `{1}{B}{B}` optionally gives target creature -1/-1
/// until end of turn (killing a 1/1 or shrinking a 2/2 until cleanup before the
/// cycling draw), and declining leaves the target unharmed.
#[test]
fn death_pulse_cycling_shrinks_by_one_until_eot_or_declines() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(); 6])
        .battlefield(1, &[grizzly_bears(), llanowar_elves()])
        .hand(0, &[death_pulse(), death_pulse()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p1, grizzly_bears()).expect("P1 Bear");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("P1 Elf");
    let lands = all_on_battlefield(&engine, p0, swamp());
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // 1. Cycle and accept targeting 2/2 Bear -> becomes 1/1 before draw.
    tap_mana_where(&mut engine, p0, |id| lands[..3].contains(&id));
    activate(&mut engine, p0, death_pulse(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim(&mut engine, &[bear], &[]);
    pass_until(&mut engine, |e| matches!(e.pending(), Pending::YesNo { .. }));
    yes_no(&mut engine, true);
    assert_eq!(pt(&engine, bear), (1, 1));
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1,
        "shrink trigger resolved before cycling draw"
    );
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );

    // 2. Cycle and decline targeting 1/1 Elf -> Elf survives, still draws.
    tap_mana_where(&mut engine, p0, |id| lands[3..6].contains(&id));
    activate(&mut engine, p0, death_pulse(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim(&mut engine, &[elf], &[]);
    pass_until(&mut engine, |e| matches!(e.pending(), Pending::YesNo { .. }));
    yes_no(&mut engine, false);
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(zone_of(&engine, elf), Zone::Battlefield);
    assert_eq!(pt(&engine, elf), (1, 1));

    // Cleanup restores Bear to 2/2.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(pt(&engine, bear), (2, 2));
}
