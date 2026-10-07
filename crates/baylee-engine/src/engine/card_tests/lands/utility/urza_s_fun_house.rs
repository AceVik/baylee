//! `cards/lands/utility/urza_s_fun_house.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Urza's Fun House` prints `{{T}}: Add {{C}}.`, `{{T}}: Add {{∞}}. Activate only once and only if you control an Urza's Mine, an Urza's Power-Plant, and an Urza's Tower.`, and `{{7}}, {{T}}: Head to AskUrza.com and click Urza's Fun House.`
///
/// Under `Coverage::Partial`, only the colorless mana ability is implemented because infinite mana, per-game limits, multi-name conditions, and out-of-game actions are unsupported.
/// With floating `{{7}}` mana from basic forests, ability 0 is offered while both unsupported abilities are omitted from `legal.abilities`.
/// Activating ability 0 adds one colorless mana to the pool and leaves `Urza's Fun House` tapped.
#[test]
fn urza_s_fun_house_taps_for_colorless_and_omits_unsupported_abilities() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                urza_s_fun_house(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let fun_house =
        on_battlefield(&engine, p0, urza_s_fun_house()).expect("fun house on battlefield");

    // Float {{7}} green mana from basic forests while keeping Urza's Fun House untapped.
    tap_mana_except(&mut engine, p0, fun_house);
    assert_eq!(engine.state().players[0].mana_pool.total(), 7);
    assert!(!is_tapped(&engine, fun_house));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(fun_house, 0)),
        "ability 0 ({{T}}: Add {{C}}) is offered"
    );
    assert!(
        !legal.abilities.contains(&(fun_house, 1)),
        "Tron ability is omitted under `Coverage::Partial`"
    );
    assert!(
        !legal.abilities.contains(&(fun_house, 2)),
        "AskUrza ability is omitted under `Coverage::Partial` despite floating {{7}}"
    );

    activate(&mut engine, p0, urza_s_fun_house(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert_eq!(pool.available(ManaColor::Green), 7);
    assert_eq!(pool.total(), 8);
    assert!(is_tapped(&engine, fun_house));
}
