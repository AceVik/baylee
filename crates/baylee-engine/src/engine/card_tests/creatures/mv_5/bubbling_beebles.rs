//! `cards/creatures/mv_5/bubbling_beebles.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn bubbling_beebles() -> CardIndex {
    card_index("75903c3b-2dbe-4662-b6a8-39599e49adde")
}

#[test]
fn bubbling_beebles_does_not_use_an_arbitrary_opponents_enchantments() {
    use crate::engine::card_tests::legends_kit as lk;
    let mut engine = lk::game(
        SEED,
        &[bubbling_beebles()],
        &[sylvan_library(), gray_ogre()],
        &[],
        &[],
    );
    let beebles = lk::obj(&engine, lk::P0, bubbling_beebles());
    let blocker = lk::obj(&engine, lk::P1, gray_ogre());
    let offer = lk::attack(&mut engine, &[beebles]);
    assert!(
        lk::may_block(&offer, blocker, beebles),
        "unsupported evasion is omitted"
    );
}

#[test]
fn creature_body_remains_playable() {
    use crate::engine::card_tests::legends_kit as lk;
    let mut engine = lk::game(SEED, &[], &[], &[bubbling_beebles()], &[]);
    lk::float(&mut engine, lk::P0, &[(ManaColor::Blue, 6)]);
    cast_with_floating(&mut engine, lk::P0, bubbling_beebles());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, lk::P0, bubbling_beebles()).is_some());
}
