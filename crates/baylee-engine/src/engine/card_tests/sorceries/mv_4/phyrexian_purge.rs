//! `cards/sorceries/mv_4/phyrexian_purge.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "df032801-fccf-4bff-aa88-d01650b6c81e"
fn phyrexian_purge() -> CardIndex {
    card_index("df032801-fccf-4bff-aa88-d01650b6c81e")
}

#[test]
fn phyrexian_purge_destroys_target_creature() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[index::GRIZZLY_BEARS],
        &[phyrexian_purge()],
        &[],
    );
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    lk::float(
        &mut engine,
        P0,
        &[
            (ManaColor::Colorless, 2),
            (ManaColor::Black, 1),
            (ManaColor::Red, 1),
        ],
    );
    lk::cast_at(&mut engine, P0, phyrexian_purge(), &[bears], &[]);

    assert!(on_battlefield(&engine, P1, index::GRIZZLY_BEARS).is_none());
}
