//! `cards/enchantments/mv_2/battle_strain.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn battle_strain() -> CardIndex {
    card_index("c4d49ecc-8650-4e91-9a39-a4a25213a5f7")
}

fn horned_turtle() -> CardIndex {
    card_index("7a4ae76b-6241-4fa8-b239-5ec941a0e7df")
}

/// Whenever any creature blocks (either side), Battle Strain deals 1 damage to
/// that blocking creature's controller; attacking does not trigger it.
#[test]
fn battle_strain_deals_one_damage_to_blocking_creatures_controller() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[battle_strain(), grizzly_bears(), grizzly_bears()],
        &[horned_turtle(), llanowar_elves()],
        &[],
        &[],
    );
    let my_bears: Vec<_> = all_on_battlefield(&e, p0, grizzly_bears());
    let opp_turtle = lk::obj(&e, p1, horned_turtle());
    let opp_elves = lk::obj(&e, p1, llanowar_elves());

    // Unblocked attack triggers nothing.
    lk::attack(&mut e, &[my_bears[0]]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, p0), 20);
    assert_eq!(lk::life(&e, p1), 18);

    // P1 attacks; untapped my_bears[1] blocks -> Battle Strain deals 1 to P0.
    reach_their_main_phase(&mut e, p1);
    let opp_attack = lk::attack(&mut e, &[opp_turtle]);
    assert!(lk::may_block(&opp_attack, my_bears[1], opp_turtle));
    lk::block(&mut e, &[(my_bears[1], opp_turtle)]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, p0), 19);

    reach_their_main_phase(&mut e, p1);
    reach_their_main_phase(&mut e, p0);
    let p0_bears: Vec<_> = all_on_battlefield(&e, p0, grizzly_bears());
    lk::attack(&mut e, &p0_bears);
    lk::block(
        &mut e,
        &[(opp_turtle, p0_bears[0]), (opp_elves, p0_bears[1])],
    );
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, p1), 16);
}
