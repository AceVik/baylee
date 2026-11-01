//! `cards/creatures/mv_4/cinder_seer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn cinder_seer() -> CardIndex {
    card_index("64b63797-9db0-41fa-b2eb-fbf51de0e337")
}

#[test]
fn cinder_seer_does_not_damage_from_an_unrevealed_hand() {
    use crate::engine::card_tests::legends_kit as lk;
    let mut engine = lk::game(
        SEED,
        &[cinder_seer()],
        &[],
        &[lightning_bolt(), lightning_bolt()],
        &[],
    );
    lk::float(&mut engine, lk::P0, &[(ManaColor::Red, 10)]);
    assert!(!lk::offered(&engine, cinder_seer(), lk::P0, 0));
    assert_eq!(lk::life(&engine, lk::P1), 20);
}

#[test]
fn creature_body_remains_playable() {
    use crate::engine::card_tests::legends_kit as lk;
    let mut engine = lk::game(SEED, &[], &[], &[cinder_seer()], &[]);
    lk::float(&mut engine, lk::P0, &[(ManaColor::Red, 6)]);
    cast_with_floating(&mut engine, lk::P0, cinder_seer());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, lk::P0, cinder_seer()).is_some());
}
