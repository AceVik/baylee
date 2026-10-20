//! `cards/enchantments/mv_4/flooded_woodlands.rs`, played.
//!
//! Oracle:
//! - "Green creatures can't attack unless their controller sacrifices a land of their choice for each green creature they control that's attacking."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);

fn flooded_woodlands() -> CardIndex {
    card_index("413172aa-b508-4e67-b68e-6a749d2e8130")
}

#[test]
fn flooded_woodlands_casts_onto_the_battlefield() {
    let mut engine = lk::game(SEED, &[], &[], &[flooded_woodlands()], &[]);

    lk::float(
        &mut engine,
        P0,
        &[
            (ManaColor::Colorless, 2),
            (ManaColor::Blue, 1),
            (ManaColor::Black, 1),
        ],
    );
    cast_with_floating(&mut engine, P0, flooded_woodlands());
    lk::settle(&mut engine);

    assert!(on_battlefield(&engine, P0, flooded_woodlands()).is_some());
}
