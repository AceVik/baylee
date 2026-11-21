//! `cards/sorceries/mv_6/slice_and_dice.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn slice_and_dice() -> CardIndex {
    card_index("463fc961-d34e-4f40-b383-5b78a0fcb5c8")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Cast normally, Slice and Dice deals 4 damage to each creature across both
/// players without damaging players.
#[test]
fn slice_and_dice_cast_deals_four_to_each_creature_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[serra_angel()])
        .hand(0, &[slice_and_dice()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf0 = on_battlefield(&engine, p0, llanowar_elves()).expect("P0 Elf");
    let angel1 = on_battlefield(&engine, p1, serra_angel()).expect("P1 Angel");

    let reds = all_on_battlefield(&engine, p0, mountain());
    tap_mana_where(&mut engine, p0, |id| reds.contains(&id));
    cast_with_floating(&mut engine, p0, slice_and_dice());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(lk::zone_of(&engine, elf0), Zone::Graveyard);
    assert_eq!(lk::zone_of(&engine, angel1), Zone::Graveyard);
    assert_eq!(lk::life(&engine, p0), 20);
    assert_eq!(lk::life(&engine, p1), 20);
}

/// Cycling Slice and Dice for `{2}{R}` optionally deals 1 damage to each
/// creature across both players before drawing (killing 1/1s and marking 1 on
/// 2/2s), and may be declined while still drawing.
#[test]
fn slice_and_dice_cycling_deals_one_to_each_creature_or_declines() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 6])
        .battlefield(1, &[llanowar_elves(), grizzly_bears()])
        .hand(0, &[slice_and_dice(), slice_and_dice()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf1 = on_battlefield(&engine, p1, llanowar_elves()).expect("P1 Elf");
    let bear1 = on_battlefield(&engine, p1, grizzly_bears()).expect("P1 Bear");
    let reds = all_on_battlefield(&engine, p0, mountain());
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // 1. Cycle and decline first -> Elf and Bear both take 0 damage, still draws.
    tap_mana_where(&mut engine, p0, |id| reds[..3].contains(&id));
    activate(&mut engine, p0, slice_and_dice(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    lk::yes_no(&mut engine, false);
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(lk::zone_of(&engine, elf1), Zone::Battlefield);
    assert_eq!(lk::damage_on(&engine, bear1), 0);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );

    // 2. Cycle and accept -> 1 damage to each creature BEFORE draw (kills 1/1, marks 1 on 2/2).
    tap_mana_where(&mut engine, p0, |id| reds[3..6].contains(&id));
    activate(&mut engine, p0, slice_and_dice(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    lk::yes_no(&mut engine, true);
    assert_eq!(lk::zone_of(&engine, elf1), Zone::Graveyard);
    assert_eq!(lk::damage_on(&engine, bear1), 1);
    assert_eq!(lk::life(&engine, p0), 20);
    assert_eq!(lk::life(&engine, p1), 20);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1,
        "sweep trigger resolved before cycling draw"
    );
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );
}
