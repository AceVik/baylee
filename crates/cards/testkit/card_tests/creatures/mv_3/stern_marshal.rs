//! `cards/creatures/mv_3/stern_marshal.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_engine::turn::Step;

fn stern_marshal() -> CardIndex {
    card_index("00602959-e9a1-4706-a1cb-70156a6fc713")
}

/// Stern Marshal ({2}{W} 2/2 Human Soldier):
/// "{T}: Target creature gets +2/+2 until end of turn. Activate only during
/// your turn, before attackers are declared."
#[test]
fn stern_marshal_pumps_target_creature_before_attackers_until_end_of_turn() {
    let mut e = lk::game(SEED, &[stern_marshal(), llanowar_elves()], &[], &[], &[]);
    let elf = lk::obj(&e, lk::P0, llanowar_elves());

    lk::to_step(&mut e, lk::P0, Step::CombatBegin);
    assert!(lk::offered(&e, stern_marshal(), lk::P0, 0));
    lk::use_ability(&mut e, lk::P0, stern_marshal(), 0, &[elf], &[]);
    assert_eq!(pt(&e, elf), (3, 3));

    let _ = lk::attack(&mut e, &[elf]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, lk::P1), 17);

    reach_their_main_phase(&mut e, lk::P1);
    assert_eq!(pt(&e, elf), (1, 1));
}

#[test]
fn stern_marshal_cannot_activate_after_attackers_or_on_opponents_turn() {
    let mut e = lk::game(SEED, &[stern_marshal(), llanowar_elves()], &[], &[], &[]);
    let elf = lk::obj(&e, lk::P0, llanowar_elves());
    assert!(lk::offered(&e, stern_marshal(), lk::P0, 0));

    let _ = lk::attack(&mut e, &[elf]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert!(!lk::offered(&e, stern_marshal(), lk::P0, 0));

    reach_their_main_phase(&mut e, lk::P1);
    e.apply(lk::P1, PlayerAction::PassPriority).unwrap();
    assert!(!lk::offered(&e, stern_marshal(), lk::P0, 0));
}
