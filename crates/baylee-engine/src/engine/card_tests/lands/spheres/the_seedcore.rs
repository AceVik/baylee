//! `cards/lands/spheres/the_seedcore.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The Seedcore prints `{{T}}: Add {{C}}.`, `{{T}}: Add one mana of any color.
/// Spend this mana only to cast Phyrexian creature spells.`, and `Corrupted — {{T}}:
/// Target 1/1 creature gets +2/+1 until end of turn. Activate only if an opponent
/// has three or more poison counters.`
///
/// Under `Coverage::Partial`, the Corrupted ability is omitted because no Condition
/// reads an opponent's poison counters. Activating
/// ability 1 prompts for a color choice via `Pending::ChooseColor` and produces one
/// restricted mana in `pool.restricted()` rather than `pool.available()`.
#[test]
fn the_seedcore_produces_restricted_phyrexian_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[the_seedcore()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let seedcore = on_battlefield(&engine, p0, the_seedcore()).expect("seedcore on battlefield");
    activate(&mut engine, p0, the_seedcore(), 1);

    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("expected ChooseColor, got {:?}", engine.pending());
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].amount, 1);
    assert_eq!(pool.restricted()[0].color, ManaColor::Green);
    assert_eq!(pool.available(ManaColor::Green), 0);
    assert_eq!(pool.total(), 1);
    assert!(is_tapped(&engine, seedcore));
}
