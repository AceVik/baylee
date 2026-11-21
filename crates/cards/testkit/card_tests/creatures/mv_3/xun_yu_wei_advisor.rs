//! `cards/creatures/mv_3/xun_yu_wei_advisor.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;
use baylee_engine::turn::Step;

fn xun_yu() -> CardIndex {
    card_index("fb5fea4a-1b04-45c6-a207-e2e8493ea968")
}

/// Xun Yu, Wei Advisor ({1}{B}{B} 1/1 Legendary Human Advisor):
/// "{T}: Target creature you control gets +2/+0 until end of turn. Activate
/// only during your turn, before attackers are declared."
#[test]
fn xun_yu_pumps_only_a_creature_you_control_plus_two_plus_zero_before_attackers() {
    let mut e = lk::game(
        SEED,
        &[xun_yu(), llanowar_elves()],
        &[index::GRIZZLY_BEARS],
        &[],
        &[],
    );
    let advisor = lk::obj(&e, lk::P0, xun_yu());
    let elf = lk::obj(&e, lk::P0, llanowar_elves());
    let their_bear = lk::obj(&e, lk::P1, index::GRIZZLY_BEARS);

    lk::to_step(&mut e, lk::P0, Step::CombatBegin);
    activate(&mut e, lk::P0, xun_yu(), 0);
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        unreachable!()
    };
    assert!(options.contains(&advisor) && options.contains(&elf));
    assert!(!options.contains(&their_bear));
    lk::aim(&mut e, &[elf], &[]);
    lk::settle(&mut e);
    assert_eq!(pt(&e, elf), (3, 1));

    let _ = lk::attack(&mut e, &[elf]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, lk::P1), 17);

    reach_their_main_phase(&mut e, lk::P1);
    assert_eq!(pt(&e, elf), (1, 1));
    e.apply(lk::P1, PlayerAction::PassPriority).unwrap();
    assert!(!lk::offered(&e, xun_yu(), lk::P0, 0));

    reach_their_main_phase(&mut e, lk::P0);
    let _ = lk::attack(&mut e, &[elf]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert!(!lk::offered(&e, xun_yu(), lk::P0, 0));
}
