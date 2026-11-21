//! `cards/creatures/mv_3/spined_basher.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_engine::turn::Step;

fn spined_basher() -> CardIndex {
    card_index("4891923b-9c8c-4816-9623-d9362e4eb503")
}

/// Spined Basher ({2}{B} 3/1 Zombie Beast):
/// "Morph {2}{B}"
#[test]
fn spined_basher_casts_face_down_attacks_and_turns_face_up_for_three_combat_damage() {
    let mut e = lk::game(SEED, &[swamp(); 6], &[], &[spined_basher()], &[]);
    cast_from_hand(&mut e, lk::P0, spined_basher());
    let slot = choose_cast_kind(&e, CastModeKind::Disguise);
    e.apply(lk::P0, PlayerAction::ChooseMode(slot)).unwrap();
    lk::settle(&mut e);

    let id = lk::obj(&e, lk::P0, spined_basher());
    assert!(
        e.state()
            .object(id)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );
    assert_eq!(pt(&e, id), (2, 2));

    // Next P0 turn: attack as 2/2, then turn face up for {2}{B} before combat damage -> hits for 3!
    reach_their_main_phase(&mut e, lk::P1);
    reach_their_main_phase(&mut e, lk::P0);
    let _ = lk::attack(&mut e, &[id]);
    lk::block(&mut e, &[]);
    lk::to_step(&mut e, lk::P0, Step::DeclareBlockers);

    lk::float(&mut e, lk::P0, &[(ManaColor::Black, 3)]);
    e.apply(
        lk::P0,
        PlayerAction::ActivateAbility {
            source: id,
            ability_index: baylee_engine::choice::TURN_FACE_UP,
        },
    )
    .expect("Morph {2}{B} turns Spined Basher face up without using the stack");
    assert!(
        !e.state()
            .object(id)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );
    assert_eq!(pt(&e, id), (3, 1));

    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, lk::P1), 17);
}
