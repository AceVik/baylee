//! `cards/creatures/mv_3/pang_tong_young_phoenix.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;
use baylee_engine::turn::Step;

fn pang_tong() -> CardIndex {
    card_index("6d9dc632-414e-480c-85ef-c2b98a6460ad")
}

/// Pang Tong, "Young Phoenix" ({1}{W}{W} 1/2 Legendary Human Advisor):
/// "{T}: Target creature gets +0/+2 until end of turn. Activate only during
/// your turn, before attackers are declared."
#[test]
fn pang_tong_grants_plus_zero_plus_two_before_attackers_until_end_of_turn() {
    let mut e = lk::game(
        SEED,
        &[pang_tong(), llanowar_elves()],
        &[index::GRIZZLY_BEARS],
        &[],
        &[],
    );
    let elf = lk::obj(&e, lk::P0, llanowar_elves());
    let bear = lk::obj(&e, lk::P1, index::GRIZZLY_BEARS);

    lk::to_step(&mut e, lk::P0, Step::CombatBegin);
    lk::use_ability(&mut e, lk::P0, pang_tong(), 0, &[elf], &[]);
    assert_eq!(pt(&e, elf), (1, 3));

    let _ = lk::attack(&mut e, &[elf]);
    lk::block(&mut e, &[(bear, elf)]);
    lk::through_combat(&mut e);
    assert!(on_battlefield(&e, lk::P0, llanowar_elves()).is_some());

    // Next turn, verify timing restrictions.
    reach_their_main_phase(&mut e, lk::P1);
    assert_eq!(pt(&e, elf), (1, 1));
    e.apply(lk::P1, PlayerAction::PassPriority).unwrap();
    assert!(!lk::offered(&e, pang_tong(), lk::P0, 0));

    reach_their_main_phase(&mut e, lk::P0);
    assert!(lk::offered(&e, pang_tong(), lk::P0, 0));
    let _ = lk::attack(&mut e, &[elf]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert!(!lk::offered(&e, pang_tong(), lk::P0, 0));
}
