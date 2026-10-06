//! `cards/lands/utility/nykthos_shrine_to_nyx.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Nykthos, Shrine to Nyx prints `{{T}}: Add {{C}}.` and `{{2}}, {{T}}: Choose a color. Add an amount of mana of that color equal to your devotion to that color.`
///
/// Under `Coverage::Partial`, the devotion ability is omitted because devotion requires counting mana symbols of a chosen color among permanents you control, which is unsupported.
/// With Nykthos and two basic lands (`forest()`) on the battlefield under `PlayerId::new(0)`, floating `{{2}}` while keeping Nykthos untapped shows that ability 0 is offered while ability 1 is withheld from `legal.abilities`.
/// Activating ability 0 adds one colorless mana to the pool and taps Nykthos.
#[test]
fn nykthos_shrine_to_nyx_taps_for_colorless_and_omits_devotion_ability() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[nykthos_shrine_to_nyx(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let nykthos =
        on_battlefield(&engine, p0, nykthos_shrine_to_nyx()).expect("nykthos on battlefield");

    // Float {{2}} from Forests while keeping Nykthos untapped.
    tap_mana_except(&mut engine, p0, nykthos);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);
    assert!(!is_tapped(&engine, nykthos));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(nykthos, 0)),
        "ability 0 ({{T}}: Add {{C}}) is offered"
    );
    assert!(
        !legal.abilities.contains(&(nykthos, 1)),
        "ability 1 is omitted under `Coverage::Partial` even with {{2}} floating"
    );

    activate(&mut engine, p0, nykthos_shrine_to_nyx(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert_eq!(pool.available(ManaColor::Green), 2);
    assert_eq!(pool.total(), 3);
    assert!(is_tapped(&engine, nykthos));
}
