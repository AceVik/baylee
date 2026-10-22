//! `cards/creatures/artifacts/mv_1/phyrexian_dreadnought.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);

// oracle_id = "a7950edb-5670-46fd-a30e-0fe888a5a32b"
fn phyrexian_dreadnought() -> CardIndex {
    card_index("a7950edb-5670-46fd-a30e-0fe888a5a32b")
}

#[test]
fn phyrexian_dreadnought_enters_and_sacrifices_itself() {
    let mut engine = lk::game(SEED, &[], &[], &[phyrexian_dreadnought()], &[]);

    lk::float(&mut engine, P0, &[(ManaColor::Colorless, 1)]);
    cast_with_floating(&mut engine, P0, phyrexian_dreadnought());
    lk::settle(&mut engine);

    assert!(on_battlefield(&engine, P0, phyrexian_dreadnought()).is_none());
    assert!(in_graveyard(&engine, P0, phyrexian_dreadnought()).is_some());
}
