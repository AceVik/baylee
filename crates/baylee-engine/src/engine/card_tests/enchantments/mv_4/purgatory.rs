//! `cards/enchantments/mv_4/purgatory.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);

// oracle_id = "2fb8e100-58a1-4dc6-bc38-ad7f115e3e46"
fn purgatory() -> CardIndex {
    card_index("2fb8e100-58a1-4dc6-bc38-ad7f115e3e46")
}

#[test]
fn purgatory_enters_the_battlefield() {
    let mut engine = lk::game(SEED, &[], &[], &[purgatory()], &[]);

    lk::float(
        &mut engine,
        P0,
        &[
            (ManaColor::Colorless, 2),
            (ManaColor::White, 1),
            (ManaColor::Black, 1),
        ],
    );
    cast_with_floating(&mut engine, P0, purgatory());
    lk::settle(&mut engine);

    assert!(on_battlefield(&engine, P0, purgatory()).is_some());
}
