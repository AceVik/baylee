//! `cards/lands/great_hall_of_the_citadel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Great Hall of the Citadel: "{T}: Add {C}." / "{1}, {T}: Add two mana in any combination of colors. Spend this mana only to cast legendary spells."
/// Paying {1} with a basic Forest and tapping the Great Hall triggers two consecutive color choices.
/// Selecting Red then White adds both restricted mana to the pool and leaves the land tapped.
#[test]
fn great_hall_of_the_citadel_filters_into_two_colored_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(115, forest())
        .battlefield(0, &[great_hall_of_the_citadel(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hall = on_battlefield(&engine, p0, great_hall_of_the_citadel()).expect("Hall deployed");
    tap_mana_except(&mut engine, p0, hall);

    activate(&mut engine, p0, great_hall_of_the_citadel(), 1);

    let Pending::ChooseColor { options: opt1, .. } = engine.pending().clone() else {
        panic!("expected first color choice, got {:?}", engine.pending());
    };
    assert_eq!(opt1.len(), 5);
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();

    let Pending::ChooseColor { options: opt2, .. } = engine.pending().clone() else {
        panic!("expected second color choice, got {:?}", engine.pending());
    };
    assert_eq!(opt2.len(), 5);
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .unwrap();

    // Read off `restricted()`, not `available()`: the printing says "spend
    // this mana only to cast legendary spells", so it never reaches the
    // plain counters — and asserting it there would pass for a card that
    // had dropped the restriction.
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 0);
    assert_eq!(pool.available(ManaColor::White), 0);
    let mut got: Vec<(ManaColor, u16)> = pool
        .restricted()
        .iter()
        .map(|m| (m.color, m.amount))
        .collect();
    got.sort_by_key(|(color, _)| *color as u8);
    let mut want = vec![(ManaColor::Red, 1u16), (ManaColor::White, 1u16)];
    want.sort_by_key(|(color, _)| *color as u8);
    assert_eq!(got, want);
    assert!(is_tapped(&engine, hall));
}
