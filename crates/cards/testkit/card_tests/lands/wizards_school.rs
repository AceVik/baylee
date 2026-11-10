//! `cards/lands/wizards_school.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Wizards' School` is a utility land under `Coverage::Implemented`.
/// It prints "{T}: Add {C}", "{1}, {T}: Add {U}", and "{2}, {T}: Add {W} or {B}."
/// When one mana is floated from an adjacent land using `tap_all_mana_but` to keep `Wizards' School`
/// untapped, activating ability 1 spends the floating mana and taps `Wizards' School` to add one blue mana.
#[test]
fn wizards_school_filters_one_mana_into_blue() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[wizards_school(), island()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let school =
        on_battlefield(&engine, p0, wizards_school()).expect("Wizards' School on battlefield");
    assert!(
        !is_tapped(&engine, school),
        "Wizards' School starts untapped"
    );

    // Float 1 mana from the Island, keeping Wizards' School untapped for its own {T} cost.
    tap_all_mana_but(&mut engine, p0, Some(wizards_school()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one mana floated from Island"
    );
    assert!(
        !is_tapped(&engine, school),
        "Wizards' School remains untapped"
    );

    // Ability 1 is "{1}, {T}: Add {U}".
    activate(&mut engine, p0, wizards_school(), 1);

    assert!(
        is_tapped(&engine, school),
        "Wizards' School tapped to pay its activation cost"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "Wizards' School produced one blue mana"
    );
}
