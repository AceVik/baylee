//! `cards/lands/utility/scrying_sheets.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Scrying Sheets prints `{{T}}: Add {{C}}.` and `{{1}}{{S}}, {{T}}: Look at the top card of your library. If that card is snow, you may reveal it and put it into your hand.`
///
/// Under `Coverage::Partial`, the top-card inspection ability is omitted because branching on printed characteristics of the top library card is unsupported.
/// With Scrying Sheets and two basic lands (`forest()`) on the battlefield under `PlayerId::new(0)`, floating two mana while keeping Scrying Sheets untapped shows that ability 0 is offered while ability 1 is withheld from `legal.abilities`.
/// Activating ability 0 adds one colorless mana to the pool and leaves Scrying Sheets tapped.
#[test]
fn scrying_sheets_taps_for_colorless_and_omits_snow_reveal_ability() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[scrying_sheets(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sheets = on_battlefield(&engine, p0, scrying_sheets()).expect("sheets on battlefield");

    // Float two mana from Forests while keeping Scrying Sheets untapped.
    tap_mana_except(&mut engine, p0, sheets);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);
    assert!(!is_tapped(&engine, sheets));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(sheets, 0)),
        "ability 0 ({{T}}: Add {{C}}) is offered"
    );
    assert!(
        !legal.abilities.contains(&(sheets, 1)),
        "ability 1 is omitted under `Coverage::Partial` even with mana floating"
    );

    activate(&mut engine, p0, scrying_sheets(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert_eq!(pool.available(ManaColor::Green), 2);
    assert_eq!(pool.total(), 3);
    assert!(is_tapped(&engine, sheets));
}
