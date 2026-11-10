//! `cards/lands/restricted/training_compound.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Training Compound: "{T}: Add {C}." / "{T}: Add {R} or {G}. Activate only if this land entered this turn or if you control a basic land."
/// Ability 0 produces {C} and leaves Training Compound tapped; the conditional
/// ability is `the_gathering_place_sentence_holds_on_dark_fortress_and_training_compound`.
#[test]
fn training_compound_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(111, forest())
        .battlefield(0, &[training_compound()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land =
        on_battlefield(&engine, p0, training_compound()).expect("Training Compound deployed");
    activate(&mut engine, p0, training_compound(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}
