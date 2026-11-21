//! `cards/creatures/mv_2/shu_farmer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_engine::turn::Step;

fn shu_farmer() -> CardIndex {
    card_index("0d059577-0790-4033-b719-a74b6a62262e")
}

/// Shu Farmer ({1}{W} 1/1 Human):
/// "{T}: You gain 1 life. Activate only during your turn, before attackers are
/// declared."
#[test]
fn shu_farmer_gains_one_life_before_attackers_and_blocks_post_attackers_or_opponents_turn() {
    let mut e = lk::game(SEED, &[shu_farmer(), llanowar_elves()], &[], &[], &[]);
    let elf = lk::obj(&e, lk::P0, llanowar_elves());

    lk::to_step(&mut e, lk::P0, Step::CombatBegin);
    assert!(lk::offered(&e, shu_farmer(), lk::P0, 0));
    lk::use_ability(&mut e, lk::P0, shu_farmer(), 0, &[], &[]);
    assert_eq!(lk::life(&e, lk::P0), 21);

    reach_their_main_phase(&mut e, lk::P1);
    e.apply(lk::P1, PlayerAction::PassPriority).unwrap();
    assert!(!lk::offered(&e, shu_farmer(), lk::P0, 0));

    reach_their_main_phase(&mut e, lk::P0);
    assert!(lk::offered(&e, shu_farmer(), lk::P0, 0));
    let _ = lk::attack(&mut e, &[elf]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert!(!lk::offered(&e, shu_farmer(), lk::P0, 0));
}
