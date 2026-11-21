//! `cards/creatures/mv_3/talas_warrior.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn talas_warrior() -> CardIndex {
    card_index("12beabf3-a84b-49c2-9462-8876a4ac74d4")
}

/// Talas Warrior ({1}{U}{U} 2/2 Human Pirate Warrior): "This creature can't be blocked."
#[test]
fn talas_warrior_cannot_be_blocked_by_ground_or_flying_creatures() {
    let mut e = lk::game(
        SEED,
        &[island(), island(), island(), llanowar_elves()],
        &[index::GRIZZLY_BEARS, index::WIND_DRAKE],
        &[talas_warrior()],
        &[],
    );
    cast_from_hand(&mut e, lk::P0, talas_warrior());
    lk::settle(&mut e);
    let warrior = lk::obj(&e, lk::P0, talas_warrior());
    let elf = lk::obj(&e, lk::P0, llanowar_elves());
    let bear = lk::obj(&e, lk::P1, index::GRIZZLY_BEARS);
    let drake = lk::obj(&e, lk::P1, index::WIND_DRAKE);

    reach_their_main_phase(&mut e, lk::P1);
    reach_their_main_phase(&mut e, lk::P0);
    let offer = lk::attack(&mut e, &[warrior, elf]);
    assert!(!lk::may_block(&offer, bear, warrior));
    assert!(!lk::may_block(&offer, drake, warrior));
    assert!(lk::may_block(&offer, bear, elf));
    assert!(lk::may_block(&offer, drake, elf));

    lk::block(&mut e, &[(bear, elf)]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, lk::P1), 18);
}
