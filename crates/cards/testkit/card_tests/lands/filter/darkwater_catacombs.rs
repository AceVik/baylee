//! `cards/lands/filter/darkwater_catacombs.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Darkwater Catacombs prints one line: "{1}, {T}: Add {U}{B}." A filter
/// land is the one mana source `tap_all_mana` may not press — the price is
/// more than its own tap symbol — so this scenario pays the {1} by hand out
/// of two tapped Forests and then reads both halves of the sentence off the
/// pool: the generic is really spent (green 2 → 1) and both {U} and {B}
/// arrive, which a land printing only "{T}: Add {U}{B}" could not produce.
/// The offer is asserted on both sides of the payment, because the engine
/// filters `abilities` through `can_afford`: the same ability is absent on an
/// empty pool and present the moment the mana is floating.
#[test]
fn darkwater_catacombs_charges_one_and_taps_for_blue_and_black() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1301, forest())
        .battlefield(0, &[darkwater_catacombs(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let catacombs =
        on_battlefield(&engine, p0, darkwater_catacombs()).expect("the land is on the table");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(catacombs, 0)),
        "the {{1}} is part of the price, and an empty pool cannot pay it: {:?}",
        legal.abilities
    );

    // The two Forests and not the Catacombs: `{1}, {T}` is more than its own
    // tap, so the helper leaves it standing for the activation below.
    tap_all_mana_but(&mut engine, p0, Some(darkwater_catacombs()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Forests, and the Catacombs untouched"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(catacombs, 0)),
        "with the mana floating the printed line is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, darkwater_catacombs(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1, "one blue");
    assert_eq!(pool.available(ManaColor::Black), 1, "and one black");
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "and one of the two Forests was spent on the {{1}}"
    );
    assert_eq!(pool.total(), 3, "nothing else came with it");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(is_tapped(&engine, catacombs), "{{T}} was paid");
}
