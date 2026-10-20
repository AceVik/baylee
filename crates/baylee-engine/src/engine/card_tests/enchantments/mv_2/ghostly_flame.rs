//! `cards/enchantments/mv_2/ghostly_flame.rs`, played.
//!
//! Oracle:
//! - "Black and/or red permanents and spells are colorless sources of damage."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);

fn ghostly_flame() -> CardIndex {
    card_index("6a461ca3-7923-4df4-9f15-4072545da117")
}

#[test]
fn ghostly_flame_casts_and_enters_battlefield() {
    let mut engine = lk::game(SEED, &[], &[], &[ghostly_flame()], &[]);
    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Black, 1), (ManaColor::Red, 1)],
    );
    lk::cast_at(&mut engine, P0, ghostly_flame(), &[], &[]);
    assert!(on_battlefield(&engine, P0, ghostly_flame()).is_some());
}
