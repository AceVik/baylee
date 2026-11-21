//! `cards/creatures/mv_1/goblin_firestarter.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_engine::turn::Step;

fn firestarter() -> CardIndex {
    card_index("18279733-5568-427c-b0a2-044bca28f688")
}

/// Goblin Firestarter ({R} 1/1 Goblin):
/// "Sacrifice this creature: It deals 1 damage to any target. Activate only
/// during your turn, before attackers are declared."
#[test]
fn goblin_firestarter_sacrifices_without_tapping_to_deal_one_damage_to_creature_or_player() {
    // Summoning sickness does not stop a tapless sacrifice cost: cast from hand
    // and sacrifice in the same main phase to kill a 1/1 creature.
    let mut e = lk::game(
        SEED,
        &[mountain()],
        &[llanowar_elves()],
        &[firestarter()],
        &[],
    );
    let elf = lk::obj(&e, lk::P1, llanowar_elves());
    cast_from_hand(&mut e, lk::P0, firestarter());
    lk::settle(&mut e);

    lk::use_ability(&mut e, lk::P0, firestarter(), 0, &[elf], &[]);
    assert!(on_battlefield(&e, lk::P0, firestarter()).is_none());
    assert!(in_graveyard(&e, lk::P0, firestarter()).is_some());
    assert!(on_battlefield(&e, lk::P1, llanowar_elves()).is_none());

    // Any target also reaches a player.
    let mut e2 = lk::game(SEED, &[firestarter()], &[], &[], &[]);
    lk::use_ability(&mut e2, lk::P0, firestarter(), 0, &[], &[lk::P1]);
    assert_eq!(lk::life(&e2, lk::P1), 19);
}

#[test]
fn goblin_firestarter_is_only_activatable_on_your_turn_before_declare_attackers() {
    let mut e = lk::game(SEED, &[firestarter(), llanowar_elves()], &[], &[], &[]);
    let elf = lk::obj(&e, lk::P0, llanowar_elves());
    assert!(lk::offered(&e, firestarter(), lk::P0, 0));

    // Still legal in beginning of combat (before DeclareAttackers).
    lk::to_step(&mut e, lk::P0, Step::CombatBegin);
    assert!(lk::offered(&e, firestarter(), lk::P0, 0));

    // Declare elf as attacker so combat reaches DeclareBlockers / CombatDamage:
    // at and after DeclareAttackers, Firestarter's ability is no longer legal.
    let _ = lk::attack(&mut e, &[elf]);
    lk::block(&mut e, &[]);
    pass_until(
        &mut e,
        |eng| matches!(eng.pending(), Pending::Priority { player, .. } if *player == lk::P0),
    );
    assert!(!lk::offered(&e, firestarter(), lk::P0, 0));

    lk::through_combat(&mut e);
    assert!(!lk::offered(&e, firestarter(), lk::P0, 0));

    // Not legal on the opponent's turn either, even before their attackers.
    reach_their_main_phase(&mut e, lk::P1);
    e.apply(lk::P1, PlayerAction::PassPriority).unwrap();
    assert!(!lk::offered(&e, firestarter(), lk::P0, 0));
}
