//! `cards/instants/mv_4/choking_tethers.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
#[allow(clippy::wildcard_imports)]
use baylee_card_testkit::card_tests::legends_kit::*;

fn choking_tethers() -> CardIndex {
    card_index("a60348fa-b182-4667-9e38-15bdf3511f82")
}

/// Cast normally, Choking Tethers taps up to four target creatures (accepting
/// 4 targets and rejecting 5, while leaving untargeted creatures untapped).
#[test]
fn choking_tethers_cast_taps_up_to_four_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .battlefield(
            1,
            &[
                llanowar_elves(),
                llanowar_elves(),
                llanowar_elves(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[choking_tethers()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p1, llanowar_elves());
    assert_eq!(elves.len(), 5);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, choking_tethers());
    // Attempting to choose 5 targets must be rejected (max is 4).
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: elves.clone(),
                    players: vec![],
                }
            )
            .is_err(),
        "up to four targets rejects five"
    );
    aim(&mut engine, &elves[..4], &[]);
    pass_until(&mut engine, stack_is_empty);

    for &id in &elves[..4] {
        assert!(is_tapped(&engine, id));
    }
    assert!(
        !is_tapped(&engine, elves[4]),
        "fifth creature stayed untapped"
    );
}

/// Cycling Choking Tethers for `{1}{U}` optionally taps target creature before
/// drawing, and declining leaves the target untapped while still drawing.
#[test]
fn choking_tethers_cycling_taps_one_or_declines() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .battlefield(1, &[llanowar_elves(), festering_goblin()])
        .hand(0, &[choking_tethers(), choking_tethers()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("P1 Elf");
    let goblin = on_battlefield(&engine, p1, festering_goblin()).expect("P1 Goblin");
    let lands = all_on_battlefield(&engine, p0, island());
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // 1. Cycle and accept -> taps Elf BEFORE cycling draw resolves.
    tap_mana_where(&mut engine, p0, |id| id == lands[0] || id == lands[1]);
    activate(&mut engine, p0, choking_tethers(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim(&mut engine, &[elf], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    yes_no(&mut engine, true);
    assert!(is_tapped(&engine, elf));
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1,
        "tap trigger resolved before cycling draw"
    );
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );

    // 2. Cycle and decline -> Goblin remains untapped, still draws 1 card.
    tap_mana_where(&mut engine, p0, |id| id == lands[2] || id == lands[3]);
    activate(&mut engine, p0, choking_tethers(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim(&mut engine, &[goblin], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    yes_no(&mut engine, false);
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(!is_tapped(&engine, goblin));
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );
}
