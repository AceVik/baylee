//! `cards/lands/utility/hall_of_oracles.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hall of Oracles prints `{{T}}: Add {{C}}.`, `{{1}}, {{T}}: Add one mana of any color.`, and `{{T}}: Put a +1/+1 counter on target creature. Activate only as a sorcery and only if you've cast an instant or sorcery spell this turn.`
///
/// Under `Coverage::Partial`, both mana abilities are implemented while the counter-granting ability is omitted because the cast-spell gate has no condition representation.
/// With one green mana floating from a `forest()`, a creature present (`quiet_creature()`), and Hall of Oracles untapped, ability 2 is not offered in `legal.abilities`.
/// The negative is about the ability being **absent**, and it is worth
/// saying what would make it stop meaning that: the printed ability costs
/// only `{{T}}`, so it would appear here the day it is written — unless it
/// is written with its "only if you've cast an instant or sorcery spell
/// this turn" gate, which nothing on this board has done. Whoever closes
/// the gap casts a spell in this test first, or the pin passes for the
/// third reason rather than the first.
/// Activating ability 1 filters the floating mana through `Pending::ChooseColor` into one blue mana.
#[test]
fn hall_of_oracles_filters_mana_and_omits_counter_ability() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[hall_of_oracles(), forest(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hall =
        on_battlefield(&engine, p0, hall_of_oracles()).expect("hall of oracles on battlefield");

    // Float the Forest and the Elves' own {{G}} while keeping Hall of
    // Oracles untapped: two, and the omitted ability charges nothing but a
    // tap, so the floating price is the {{1}} of ability 1.
    tap_mana_except(&mut engine, p0, hall);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        2
    );
    assert!(!is_tapped(&engine, hall));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(hall, 0)),
        "ability 0 ({{T}}: Add {{C}}) is offered"
    );
    assert!(
        legal.abilities.contains(&(hall, 1)),
        "ability 1 ({{1}}, {{T}}: Add one mana of any color) is offered"
    );
    assert!(
        !legal.abilities.contains(&(hall, 2)),
        "ability 2 is omitted under `Coverage::Partial`"
    );

    activate(&mut engine, p0, hall_of_oracles(), 1);

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert_eq!(options.len(), 5);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "the {{1}} came out of the two floating greens, and one is left"
    );
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert_eq!(pool.total(), 2);
    assert!(is_tapped(&engine, hall));
}
