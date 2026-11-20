//! `cards/creatures/mv_2/gempalm_strider.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
#[allow(clippy::wildcard_imports)]
use baylee_card_testkit::card_tests::legends_kit::*;

fn gempalm_strider() -> CardIndex {
    card_index("48792146-e20c-4d93-83af-f746069fc6f1")
}

/// Casting Gempalm Strider normally seats a 2/2 Elf without buffing Elves.
#[test]
fn gempalm_strider_casts_as_2_2_elf_without_pumping() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), llanowar_elves()])
        .hand(0, &[gempalm_strider()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("Elf seated");
    cast_from_hand(&mut engine, p0, gempalm_strider());
    pass_until(&mut engine, stack_is_empty);

    let strider = on_battlefield(&engine, p0, gempalm_strider()).expect("Strider entered");
    assert_eq!(pt(&engine, strider), (2, 2));
    assert_eq!(pt(&engine, elf), (1, 1));
}

/// Cycling Gempalm Strider for `{2}{G}{G}` resolves its +2/+2 pump to all Elf
/// creatures across both players BEFORE the cycling draw, ignores non-Elves and
/// Elves entering afterward (CR 611.2c), and wears off at cleanup.
#[test]
fn gempalm_strider_cycling_pumps_elves_before_draw_and_expires_at_eot() {
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
                llanowar_elves(),
                festering_goblin(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[gempalm_strider(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf0 = on_battlefield(&engine, p0, llanowar_elves()).expect("P0 Elf");
    let goblin0 = on_battlefield(&engine, p0, festering_goblin()).expect("P0 Goblin");
    let elf1 = on_battlefield(&engine, p1, llanowar_elves()).expect("P1 Elf");
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    let lands = all_on_battlefield(&engine, p0, forest());
    tap_mana_where(&mut engine, p0, |id| lands[..4].contains(&id));
    activate(&mut engine, p0, gempalm_strider(), 0);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Stack).len(),
        2,
        "CycledThis trigger stacks above cycling draw"
    );

    // Let top stack item (CycledThis) resolve while cycling draw still waits.
    pass_until(&mut engine, |e| {
        e.state().zones.list(ZoneLocation::Stack).len() == 1
    });
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1,
        "trigger resolved before cycling draw"
    );
    assert_eq!(pt(&engine, elf0), (3, 3));
    assert_eq!(pt(&engine, elf1), (3, 3));
    assert_eq!(pt(&engine, goblin0), (1, 1), "non-Elf Goblin untouched");

    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "cycling drew 1 card"
    );

    // CR 611.2c: an Elf entering after resolution does not get +2/+2.
    tap_mana_where(&mut engine, p0, |id| id == lands[4]);
    cast_with_floating(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    let late_elf = all_on_battlefield(&engine, p0, llanowar_elves())
        .into_iter()
        .find(|id| *id != elf0)
        .expect("late Elf entered");
    assert_eq!(pt(&engine, late_elf), (1, 1));

    // Expires at cleanup.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(pt(&engine, elf0), (1, 1));
    assert_eq!(pt(&engine, elf1), (1, 1));
}
