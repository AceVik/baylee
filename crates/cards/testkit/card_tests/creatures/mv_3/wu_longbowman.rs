//! `cards/creatures/mv_3/wu_longbowman.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_engine::turn::Step;

fn wu_longbowman() -> CardIndex {
    card_index("68fc55b9-55cc-4caf-a052-b44ddc0b346f")
}

/// Wu Longbowman ({2}{U} 1/1 Human Soldier Archer):
/// "{T}: This creature deals 1 damage to any target. Activate only during your
/// turn, before attackers are declared."
#[test]
fn wu_longbowman_pings_creature_or_player_before_attackers() {
    let mut e = lk::game(SEED, &[wu_longbowman()], &[llanowar_elves()], &[], &[]);
    let elf = lk::obj(&e, lk::P1, llanowar_elves());

    lk::use_ability(&mut e, lk::P0, wu_longbowman(), 0, &[elf], &[]);
    assert!(on_battlefield(&e, lk::P1, llanowar_elves()).is_none());

    reach_their_main_phase(&mut e, lk::P1);
    reach_their_main_phase(&mut e, lk::P0);
    lk::to_step(&mut e, lk::P0, Step::CombatBegin);
    lk::use_ability(&mut e, lk::P0, wu_longbowman(), 0, &[], &[lk::P1]);
    assert_eq!(lk::life(&e, lk::P1), 19);
}

#[test]
fn wu_longbowman_cannot_activate_after_attackers_or_on_opponents_turn() {
    let mut e = lk::game(SEED, &[wu_longbowman(), llanowar_elves()], &[], &[], &[]);
    let elf = lk::obj(&e, lk::P0, llanowar_elves());
    assert!(lk::offered(&e, wu_longbowman(), lk::P0, 0));

    let _ = lk::attack(&mut e, &[elf]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert!(!lk::offered(&e, wu_longbowman(), lk::P0, 0));

    reach_their_main_phase(&mut e, lk::P1);
    e.apply(lk::P1, PlayerAction::PassPriority).unwrap();
    assert!(!lk::offered(&e, wu_longbowman(), lk::P0, 0));
}
