//! `cards/creatures/mv_2/yellow_scarves_cavalry.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn yellow_scarves_cavalry() -> CardIndex {
    card_index("8ff7e8e9-11b4-4a77-a423-70777f00482b")
}

/// Yellow Scarves Cavalry ({1}{R} 1/1 Human Soldier):
/// "Horsemanship; This creature can't block."
#[test]
fn yellow_scarves_cavalry_has_horsemanship_evasion_and_cannot_block() {
    let mut e = lk::game(
        SEED,
        &[mountain(), mountain(), index::SHU_CAVALRY],
        &[index::GRIZZLY_BEARS, index::WEI_SCOUT, index::SHU_CAVALRY],
        &[yellow_scarves_cavalry()],
        &[],
    );
    cast_from_hand(&mut e, lk::P0, yellow_scarves_cavalry());
    lk::settle(&mut e);
    let scarves = lk::obj(&e, lk::P0, yellow_scarves_cavalry());
    let my_shu = lk::obj(&e, lk::P0, index::SHU_CAVALRY);
    let their_bear = lk::obj(&e, lk::P1, index::GRIZZLY_BEARS);
    let their_scout = lk::obj(&e, lk::P1, index::WEI_SCOUT);
    let their_shu = lk::obj(&e, lk::P1, index::SHU_CAVALRY);

    // Even with Horsemanship, Yellow Scarves Cavalry cannot block a Horsemanship attacker.
    reach_their_main_phase(&mut e, lk::P1);
    let offer_def = lk::attack(&mut e, &[their_scout]);
    assert!(!lk::may_block(&offer_def, scarves, their_scout));
    assert!(lk::may_block(&offer_def, my_shu, their_scout));
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);

    // Attacking evades non-Horsemanship creatures while an untapped Horsemanship creature may block.
    reach_their_main_phase(&mut e, lk::P0);
    let offer_atk = lk::attack(&mut e, &[scarves]);
    assert!(!lk::may_block(&offer_atk, their_bear, scarves));
    assert!(lk::may_block(&offer_atk, their_shu, scarves));
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, lk::P1), 19);
}
