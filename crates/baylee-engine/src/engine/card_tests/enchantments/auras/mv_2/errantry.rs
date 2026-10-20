//! `cards/enchantments/auras/mv_2/errantry.rs`, played.
//!
//! Oracle:
//! - "Enchant creature"
//! - "Enchanted creature gets +3/+0 and can only attack alone."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn errantry() -> CardIndex {
    card_index("fee455f2-fcc4-47b1-a533-92fa1ae8ae63")
}

#[test]
fn errantry_gives_enchanted_creature_plus_three_plus_zero() {
    let mut engine = lk::game(SEED, &[index::GRIZZLY_BEARS], &[], &[errantry()], &[]);
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::Red, 1)],
    );
    lk::cast_at(&mut engine, P0, errantry(), &[bears], &[]);

    assert_eq!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .characteristics()
            .power,
        Some(5)
    );
    assert_eq!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .characteristics()
            .toughness,
        Some(2)
    );
}
