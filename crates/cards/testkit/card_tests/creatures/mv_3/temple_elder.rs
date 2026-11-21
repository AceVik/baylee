//! `cards/creatures/mv_3/temple_elder.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_engine::turn::Step;

fn temple_elder() -> CardIndex {
    card_index("a0ace0db-1bd6-4b30-b0e2-d266967707d7")
}

/// Temple Elder ({2}{W} 1/2 Human Cleric):
/// "{T}: You gain 1 life. Activate only during your turn, before attackers are
/// declared."
#[test]
fn temple_elder_gains_one_life_before_attackers_and_blocks_post_attackers_or_opponents_turn() {
    let mut e = lk::game(SEED, &[temple_elder(), llanowar_elves()], &[], &[], &[]);
    let elf = lk::obj(&e, lk::P0, llanowar_elves());

    lk::to_step(&mut e, lk::P0, Step::CombatBegin);
    assert!(lk::offered(&e, temple_elder(), lk::P0, 0));
    lk::use_ability(&mut e, lk::P0, temple_elder(), 0, &[], &[]);
    assert_eq!(lk::life(&e, lk::P0), 21);

    // Next turn, pass past DeclareAttackers without tapping Temple Elder: no longer legal.
    reach_their_main_phase(&mut e, lk::P1);
    e.apply(lk::P1, PlayerAction::PassPriority).unwrap();
    assert!(!lk::offered(&e, temple_elder(), lk::P0, 0));

    reach_their_main_phase(&mut e, lk::P0);
    assert!(lk::offered(&e, temple_elder(), lk::P0, 0));
    let _ = lk::attack(&mut e, &[elf]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert!(!lk::offered(&e, temple_elder(), lk::P0, 0));
}
