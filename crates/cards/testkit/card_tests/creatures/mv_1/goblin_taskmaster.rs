//! `cards/creatures/mv_1/goblin_taskmaster.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn goblin_taskmaster() -> CardIndex {
    card_index("64fddd5e-06e4-4d54-9ea6-0c9b82f3b153")
}

/// Goblin Taskmaster ({R} 1/1 Goblin):
/// "{1}{R}: Target Goblin creature gets +1/+0 until end of turn. Morph {R}"
#[test]
fn goblin_taskmaster_pumps_only_goblin_creatures_until_end_of_turn() {
    let mut e = lk::game(
        SEED,
        &[goblin_taskmaster(), raging_goblin(), grizzly_bears()],
        &[mons_s_goblin_raiders()],
        &[],
        &[],
    );
    let taskmaster = lk::obj(&e, lk::P0, goblin_taskmaster());
    let raging = lk::obj(&e, lk::P0, raging_goblin());
    let bear = lk::obj(&e, lk::P0, grizzly_bears());
    let opp_gob = lk::obj(&e, lk::P1, mons_s_goblin_raiders());

    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 2)]);
    activate(&mut e, lk::P0, goblin_taskmaster(), 0);
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("expected Goblin target choice, got {:?}", e.pending());
    };
    assert!(
        options.contains(&taskmaster) && options.contains(&raging) && options.contains(&opp_gob)
    );
    assert!(!options.contains(&bear), "non-Goblin is not a legal target");
    lk::aim(&mut e, &[raging], &[]);
    lk::settle(&mut e);
    assert_eq!(pt(&e, raging), (2, 1));

    reach_their_main_phase(&mut e, lk::P1);
    assert_eq!(pt(&e, raging), (1, 1), "+1/+0 expires at end of turn");
}

#[test]
fn goblin_taskmaster_casts_face_down_and_turns_face_up_for_red() {
    let mut e = lk::game(SEED, &[mountain(); 4], &[], &[goblin_taskmaster()], &[]);
    cast_from_hand(&mut e, lk::P0, goblin_taskmaster());
    let slot = choose_cast_kind(&e, CastModeKind::Disguise);
    e.apply(lk::P0, PlayerAction::ChooseMode(slot)).unwrap();
    lk::settle(&mut e);

    let id = lk::obj(&e, lk::P0, goblin_taskmaster());
    assert!(
        e.state()
            .object(id)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );
    assert_eq!(pt(&e, id), (2, 2));

    e.apply(
        lk::P0,
        PlayerAction::ActivateAbility {
            source: id,
            ability_index: baylee_engine::choice::TURN_FACE_UP,
        },
    )
    .expect("remaining {R} pays Morph {R}");
    assert!(
        !e.state()
            .object(id)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );
    assert_eq!(pt(&e, id), (1, 1));
    assert_eq!(lk::pool_total(&e, lk::P0), 0);
}
