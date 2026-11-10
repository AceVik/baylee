//! `cards/lands/restricted/shrine_of_the_forsaken_gods.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Shrine of the Forsaken Gods: "{T}: Add {C}." / "{T}: Add {C}{C}. Spend this mana only to cast colorless spells. Activate only if you control seven or more lands."
/// Under `Coverage::Implemented`, controlling seven lands enables the second mana ability.
/// Activating ability 1 adds two restricted colorless mana to `pool.restricted()`.
#[test]
fn shrine_of_the_forsaken_gods_adds_two_restricted_colorless_with_seven_lands() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(108, forest())
        .battlefield(
            0,
            &[
                shrine_of_the_forsaken_gods(),
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

    let shrine =
        on_battlefield(&engine, p0, shrine_of_the_forsaken_gods()).expect("Shrine deployed");
    activate(&mut engine, p0, shrine_of_the_forsaken_gods(), 1);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 0);
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].amount, 2);
    assert_eq!(pool.restricted()[0].color, ManaColor::Colorless);
    assert!(is_tapped(&engine, shrine));
}
