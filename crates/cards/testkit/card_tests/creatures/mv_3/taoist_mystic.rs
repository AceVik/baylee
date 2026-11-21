//! `cards/creatures/mv_3/taoist_mystic.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn taoist_mystic() -> CardIndex {
    card_index("e705ec38-b8f7-4f99-8cb9-8448dc3e50ec")
}

/// Taoist Mystic ({2}{G} 2/2 Human Mystic):
/// "This creature can't be blocked by creatures with horsemanship."
#[test]
fn taoist_mystic_cannot_be_blocked_by_horsemanship_but_is_blockable_otherwise() {
    let mut e = lk::game(
        SEED,
        &[forest(), forest(), forest()],
        &[index::SHU_CAVALRY, index::GRIZZLY_BEARS],
        &[taoist_mystic()],
        &[],
    );
    cast_from_hand(&mut e, lk::P0, taoist_mystic());
    lk::settle(&mut e);
    let mystic = lk::obj(&e, lk::P0, taoist_mystic());
    let cavalry = lk::obj(&e, lk::P1, index::SHU_CAVALRY);
    let bear = lk::obj(&e, lk::P1, index::GRIZZLY_BEARS);

    reach_their_main_phase(&mut e, lk::P1);
    reach_their_main_phase(&mut e, lk::P0);
    let offer = lk::attack(&mut e, &[mystic]);
    assert!(!lk::may_block(&offer, cavalry, mystic));
    assert!(lk::may_block(&offer, bear, mystic));

    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, lk::P1), 18);
}
