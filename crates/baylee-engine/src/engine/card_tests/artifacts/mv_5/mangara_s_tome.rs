//! `cards/artifacts/mv_5/mangara_s_tome.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);

// oracle_id = "f515c691-07be-47ea-8bc0-46221eab8b46"
fn mangara_s_tome() -> CardIndex {
    card_index("f515c691-07be-47ea-8bc0-46221eab8b46")
}

#[test]
fn mangara_s_tome_enters_battlefield_and_shuffles_library() {
    let mut engine = lk::game(SEED, &[], &[], &[mangara_s_tome()], &[]);

    lk::float(&mut engine, P0, &[(ManaColor::Colorless, 5)]);
    cast_with_floating(&mut engine, P0, mangara_s_tome());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, mangara_s_tome()).is_some());
}
