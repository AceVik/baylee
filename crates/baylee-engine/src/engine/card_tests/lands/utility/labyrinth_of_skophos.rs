//! `cards/lands/utility/labyrinth_of_skophos.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Labyrinth of Skophos: "{T}: Add {C}." / "{4}, {T}: Remove target attacking or blocking creature from combat."
/// Under `Coverage::Partial`, the ability to remove a creature from combat is unsupported and omitted.
/// The land taps for its mana ability adding {C} to the mana pool.
#[test]
fn labyrinth_of_skophos_taps_for_colorless_and_omits_combat_removal() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(127, forest())
        // Four Forests for the {4} the missing ability costs — generic mana
        // takes any colour — because the pin below is read off the offer and
        // `can_afford` reads the pool.
        .battlefield(
            0,
            &[
                labyrinth_of_skophos(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lab = on_battlefield(&engine, p0, labyrinth_of_skophos()).expect("Labyrinth deployed");

    // Read **before** the mana ability is pressed and with the price
    // floating. It used to be read after, on an empty pool and a tapped
    // land, so two separate things guaranteed the absence and neither of
    // them was the card.
    tap_mana_except(&mut engine, p0, lab);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "the {{4}} is standing in the pool"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.iter().any(|(s, ai)| *s == lab && *ai == 1),
        "no second ability is offered, and its {{4}} is paid for right here: \
         {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, labyrinth_of_skophos(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, lab));
}
