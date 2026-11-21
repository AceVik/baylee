//! `cards/creatures/mv_1/fire_bowman.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_engine::turn::Step;

fn fire_bowman() -> CardIndex {
    card_index("3f23d7d3-28c6-4388-afb5-8f101b973083")
}

/// Fire Bowman ({R} 1/1 Human Soldier Archer):
/// "Sacrifice this creature: It deals 1 damage to any target. Activate only
/// during your turn, before attackers are declared."
#[test]
fn fire_bowman_sacrifices_without_tapping_to_deal_one_damage_to_creature_or_player() {
    let mut e = lk::game(
        SEED,
        &[mountain()],
        &[llanowar_elves()],
        &[fire_bowman()],
        &[],
    );
    let elf = lk::obj(&e, lk::P1, llanowar_elves());
    cast_from_hand(&mut e, lk::P0, fire_bowman());
    lk::settle(&mut e);

    lk::use_ability(&mut e, lk::P0, fire_bowman(), 0, &[elf], &[]);
    assert!(on_battlefield(&e, lk::P0, fire_bowman()).is_none());
    assert!(in_graveyard(&e, lk::P0, fire_bowman()).is_some());
    assert!(on_battlefield(&e, lk::P1, llanowar_elves()).is_none());

    let mut e2 = lk::game(SEED, &[fire_bowman()], &[], &[], &[]);
    lk::use_ability(&mut e2, lk::P0, fire_bowman(), 0, &[], &[lk::P1]);
    assert_eq!(lk::life(&e2, lk::P1), 19);
}

#[test]
fn fire_bowman_is_only_activatable_on_your_turn_before_declare_attackers() {
    let mut e = lk::game(SEED, &[fire_bowman(), llanowar_elves()], &[], &[], &[]);
    let elf = lk::obj(&e, lk::P0, llanowar_elves());
    assert!(lk::offered(&e, fire_bowman(), lk::P0, 0));

    lk::to_step(&mut e, lk::P0, Step::CombatBegin);
    assert!(lk::offered(&e, fire_bowman(), lk::P0, 0));

    let _ = lk::attack(&mut e, &[elf]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert!(!lk::offered(&e, fire_bowman(), lk::P0, 0));

    reach_their_main_phase(&mut e, lk::P1);
    e.apply(lk::P1, PlayerAction::PassPriority).unwrap();
    assert!(!lk::offered(&e, fire_bowman(), lk::P0, 0));
}
