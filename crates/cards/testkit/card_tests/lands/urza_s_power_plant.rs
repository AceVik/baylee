//! `cards/lands/urza_s_power_plant.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Urza's Power Plant: "{T}: Add {C}. If you control an Urza's Mine and an Urza's Tower, add {C}{C} instead."
/// Alone it taps for the one {C}; the other two beside it are
/// `the_urza_lands_make_seven_together_and_one_each_without_the_third_type`.
#[test]
fn urza_s_power_plant_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(121, forest())
        .battlefield(0, &[urza_s_power_plant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let plant = on_battlefield(&engine, p0, urza_s_power_plant()).expect("Power Plant deployed");

    activate(&mut engine, p0, urza_s_power_plant(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, plant));
}

/// Urza's Mine, Urza's Power Plant and Urza's Tower: "{T}: Add {C}. If you
/// control an Urza's [the other two], add {C}{C} ({C}{C}{C} for the Tower)
/// instead."
///
/// The full set makes seven, read one land at a time: two, two, three. The
/// second board has two Mines and a Tower and no Power-Plant, so every land
/// is missing one of its two, and each makes its one {C}. That board is the
/// one a single "a Mine or a Power-Plant" filter counted at two would have
/// paid the Tower's three for.
#[test]
fn the_urza_lands_make_seven_together_and_one_each_without_the_third_type() {
    let p0 = PlayerId::new(0);
    let made = |board: &[CardIndex]| -> Vec<(CardIndex, u64)> {
        let mut engine = Duel::new(122, forest()).battlefield(0, board).start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let mut made = Vec::new();
        for id in engine.state().zones.list(ZoneLocation::Battlefield).clone() {
            let card = engine
                .state()
                .object(id)
                .and_then(|o| o.card)
                .map(|c| c.index);
            let Some(card) = card.filter(|c| board.contains(c)) else {
                continue;
            };
            let before = engine.state().players[0].mana_pool.total();
            tap_mana_where(&mut engine, p0, |x| x == id);
            let pool = &engine.state().players[0].mana_pool;
            assert_eq!(
                u64::from(pool.available(ManaColor::Colorless)),
                pool.total(),
                "colorless only"
            );
            made.push((card, pool.total() - before));
        }
        made.sort();
        made
    };

    let mut tron = vec![
        (urza_s_mine(), 2),
        (urza_s_power_plant(), 2),
        (urza_s_tower(), 3),
    ];
    tron.sort();
    assert_eq!(
        made(&[urza_s_mine(), urza_s_power_plant(), urza_s_tower()]),
        tron
    );

    let mut short = vec![(urza_s_mine(), 1), (urza_s_mine(), 1), (urza_s_tower(), 1)];
    short.sort();
    assert_eq!(
        made(&[urza_s_mine(), urza_s_mine(), urza_s_tower()]),
        short,
        "two Mines are not a Mine and a Power-Plant"
    );
}
