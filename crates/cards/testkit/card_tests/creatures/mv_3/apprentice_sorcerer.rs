//! `cards/creatures/mv_3/apprentice_sorcerer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_engine::turn::Step;

fn apprentice_sorcerer() -> CardIndex {
    card_index("e26bf39f-2d1a-4bc1-aed2-d6d4505f1018")
}

/// Apprentice Sorcerer ({2}{U} 1/1 Human Wizard Sorcerer):
/// "{T}: This creature deals 1 damage to any target. Activate only during your
/// turn, before attackers are declared."
#[test]
fn apprentice_sorcerer_pings_creature_or_player_before_attackers() {
    let mut e = lk::game(
        SEED,
        &[apprentice_sorcerer()],
        &[llanowar_elves()],
        &[],
        &[],
    );
    let elf = lk::obj(&e, lk::P1, llanowar_elves());

    lk::use_ability(&mut e, lk::P0, apprentice_sorcerer(), 0, &[elf], &[]);
    assert!(on_battlefield(&e, lk::P1, llanowar_elves()).is_none());

    reach_their_main_phase(&mut e, lk::P1);
    reach_their_main_phase(&mut e, lk::P0);
    lk::to_step(&mut e, lk::P0, Step::CombatBegin);
    lk::use_ability(&mut e, lk::P0, apprentice_sorcerer(), 0, &[], &[lk::P1]);
    assert_eq!(lk::life(&e, lk::P1), 19);
}

#[test]
fn apprentice_sorcerer_cannot_activate_after_attackers_or_on_opponents_turn() {
    let mut e = lk::game(
        SEED,
        &[apprentice_sorcerer(), llanowar_elves()],
        &[],
        &[],
        &[],
    );
    let elf = lk::obj(&e, lk::P0, llanowar_elves());
    assert!(lk::offered(&e, apprentice_sorcerer(), lk::P0, 0));

    let _ = lk::attack(&mut e, &[elf]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert!(!lk::offered(&e, apprentice_sorcerer(), lk::P0, 0));

    reach_their_main_phase(&mut e, lk::P1);
    e.apply(lk::P1, PlayerAction::PassPriority).unwrap();
    assert!(!lk::offered(&e, apprentice_sorcerer(), lk::P0, 0));
}
