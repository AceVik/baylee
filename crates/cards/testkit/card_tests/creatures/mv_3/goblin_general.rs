//! `cards/creatures/mv_3/goblin_general.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn goblin_general() -> CardIndex {
    card_index("5fab0d9c-ccf7-4c6a-94b4-959be139dad8")
}

/// Goblin General ({1}{R}{R} 1/1 Goblin Warrior):
/// "Whenever this creature attacks, Goblin creatures you control get +1/+1
/// until end of turn."
#[test]
fn goblin_general_pumps_all_goblins_you_control_when_it_attacks_until_end_of_turn() {
    let mut e = lk::game(
        SEED,
        &[goblin_general(), index::RAGING_GOBLIN, llanowar_elves()],
        &[index::RAGING_GOBLIN],
        &[],
        &[],
    );
    let general = lk::obj(&e, lk::P0, goblin_general());
    let my_goblin = lk::obj(&e, lk::P0, index::RAGING_GOBLIN);
    let my_elf = lk::obj(&e, lk::P0, llanowar_elves());
    let their_goblin = lk::obj(&e, lk::P1, index::RAGING_GOBLIN);

    // Attack with Goblin General alone: trigger goes on stack, resolves before blockers.
    pass_until(&mut e, |eng| {
        matches!(eng.pending(), Pending::ChooseAttackers { .. })
    });
    e.apply(
        lk::P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(general, Defender::Player(lk::P1))],
        },
    )
    .unwrap();
    pass_until(&mut e, |eng| {
        matches!(eng.pending(), Pending::ChooseBlockers { .. })
    });

    assert_eq!(pt(&e, general), (2, 2));
    assert_eq!(pt(&e, my_goblin), (2, 2));
    assert_eq!(pt(&e, my_elf), (1, 1));
    assert_eq!(pt(&e, their_goblin), (1, 1));

    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, lk::P1), 18);

    reach_their_main_phase(&mut e, lk::P1);
    assert_eq!(pt(&e, general), (1, 1));
    assert_eq!(pt(&e, my_goblin), (1, 1));
}
