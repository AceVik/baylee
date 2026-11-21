//! `cards/creatures/mv_3/zhuge_jin_wu_strategist.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;
use baylee_engine::turn::Step;

fn zhuge_jin() -> CardIndex {
    card_index("ce995d3f-be7c-4d2a-8ec4-c7f7f831f0bf")
}

/// Zhuge Jin, Wu Strategist ({1}{U}{U} 1/1 Legendary Human Advisor):
/// "{T}: Target creature can't be blocked this turn. Activate only during your
/// turn, before attackers are declared."
#[test]
fn zhuge_jin_makes_target_creature_unblockable_before_attackers_until_end_of_turn() {
    let mut e = lk::game(
        SEED,
        &[zhuge_jin(), llanowar_elves()],
        &[index::GRIZZLY_BEARS],
        &[],
        &[],
    );
    let elf = lk::obj(&e, lk::P0, llanowar_elves());
    let bear = lk::obj(&e, lk::P1, index::GRIZZLY_BEARS);

    lk::to_step(&mut e, lk::P0, Step::CombatBegin);
    lk::use_ability(&mut e, lk::P0, zhuge_jin(), 0, &[elf], &[]);

    let offer = lk::attack(&mut e, &[elf]);
    assert!(!lk::may_block(&offer, bear, elf));
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, lk::P1), 19);

    // Opponent's turn and post-attackers timing checks, plus expiry of unblockable.
    reach_their_main_phase(&mut e, lk::P1);
    e.apply(lk::P1, PlayerAction::PassPriority).unwrap();
    assert!(!lk::offered(&e, zhuge_jin(), lk::P0, 0));

    reach_their_main_phase(&mut e, lk::P0);
    assert!(lk::offered(&e, zhuge_jin(), lk::P0, 0));
    let offer2 = lk::attack(&mut e, &[elf]);
    assert!(lk::may_block(&offer2, bear, elf));
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert!(!lk::offered(&e, zhuge_jin(), lk::P0, 0));
}
