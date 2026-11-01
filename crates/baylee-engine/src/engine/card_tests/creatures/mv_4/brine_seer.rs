//! `cards/creatures/mv_4/brine_seer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn brine_seer() -> CardIndex {
    card_index("64c88c94-3a9d-4fd4-b58c-4d2167c2b8d5")
}

#[test]
fn brine_seer_does_not_counter_from_an_unrevealed_hand() {
    use crate::engine::card_tests::legends_kit as lk;
    let mut engine = lk::game(
        SEED,
        &[mountain(), mountain(), mountain()],
        &[brine_seer()],
        &[gray_ogre()],
        &[counterspell()],
    );
    cast_from_hand(&mut engine, lk::P0, gray_ogre());
    engine.apply(lk::P0, PlayerAction::PassPriority).unwrap();
    lk::float(&mut engine, lk::P1, &[(ManaColor::Blue, 10)]);
    assert!(!lk::offered(&engine, brine_seer(), lk::P1, 0));
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, lk::P0, gray_ogre()).is_some());
}

#[test]
fn creature_body_remains_playable() {
    use crate::engine::card_tests::legends_kit as lk;
    let mut engine = lk::game(SEED, &[], &[], &[brine_seer()], &[]);
    lk::float(&mut engine, lk::P0, &[(ManaColor::Blue, 6)]);
    cast_with_floating(&mut engine, lk::P0, brine_seer());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, lk::P0, brine_seer()).is_some());
}
