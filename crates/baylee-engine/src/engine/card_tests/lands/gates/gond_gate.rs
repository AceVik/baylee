//! `cards/lands/gates/gond_gate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Gond Gate: "Gates you control enter untapped." / "{T}: Add {C}." / "{T}: Add one mana of any color that a Gate you control could produce."
/// Under `Coverage::Partial`, the Gate-untap modifier and Gate-dependent colored mana are omitted.
/// Activating Gond Gate's implemented ability adds {C} to the mana pool and taps it.
#[test]
fn gond_gate_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(125, forest())
        .battlefield(0, &[gond_gate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p0, gond_gate()).expect("Gond Gate deployed");
    activate(&mut engine, p0, gond_gate(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}
