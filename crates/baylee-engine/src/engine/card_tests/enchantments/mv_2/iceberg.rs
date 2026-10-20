//! `cards/enchantments/mv_2/iceberg.rs`, played.
//!
//! Oracle:
//! - "This enchantment enters with X ice counters on it."
//! - "{3}: Put an ice counter on this enchantment."
//! - "Remove an ice counter from this enchantment: Add {C}."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);

fn iceberg() -> CardIndex {
    card_index("cff3fba2-9c5d-4e98-9ff0-0cc0b3bc8dbe")
}

#[test]
fn iceberg_enters_with_x_ice_counters_adds_more_and_removes_for_colorless_mana() {
    let mut engine = lk::game(SEED, &[], &[], &[iceberg()], &[]);
    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 2), (ManaColor::Blue, 2)],
    );
    lk::announce(&mut engine, P0, iceberg(), 0, 2, &[], &[]);
    lk::settle(&mut engine);

    let berg = lk::obj(&engine, P0, iceberg());
    assert_eq!(
        engine
            .state()
            .object(berg)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::ICE),
        2
    );

    // Pay {3} to put another ice counter on Iceberg (ability 0).
    lk::float(&mut engine, P0, &[(ManaColor::Colorless, 3)]);
    lk::use_ability(&mut engine, P0, iceberg(), 0, &[], &[]);
    assert_eq!(
        engine
            .state()
            .object(berg)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::ICE),
        3
    );

    // Remove an ice counter to add {C} (ability 1).
    activate(&mut engine, P0, iceberg(), 1);
    assert_eq!(
        engine
            .state()
            .object(berg)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::ICE),
        2
    );
    assert_eq!(lk::pool_of(&engine, P0, ManaColor::Colorless), 1);
}
