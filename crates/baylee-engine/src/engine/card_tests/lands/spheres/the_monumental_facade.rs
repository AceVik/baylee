//! `cards/lands/spheres/the_monumental_facade.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The Monumental Facade prints `This land enters with two oil counters on it.`,
/// `{{T}}: Add {{C}}.`, and `{{T}}, Remove an oil counter from this land: Put an oil
/// counter on target artifact or creature you control. Activate only as a sorcery.`
///
/// Under `Coverage::Partial`, oil counters have no `CounterKind` constant in the DSL,
/// so the counter-related abilities are omitted. Activating ability 0 adds one colorless
/// mana to the pool and taps the land.
#[test]
fn the_monumental_facade_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[the_monumental_facade()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let facade =
        on_battlefield(&engine, p0, the_monumental_facade()).expect("facade on battlefield");
    activate(&mut engine, p0, the_monumental_facade(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert_eq!(pool.total(), 1);
    assert!(is_tapped(&engine, facade));
}
