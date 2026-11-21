//! `cards/creatures/mv_2/headhunter.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn headhunter() -> CardIndex {
    card_index("83578de7-5488-4952-af47-8fe966897210")
}

/// Headhunter ({1}{B} 1/1 Human Cleric):
/// "Whenever this creature deals combat damage to a player, that player
/// discards a card. Morph {B}"
#[test]
fn headhunter_forces_discard_on_combat_damage_to_player_not_blocker() {
    let mut e = lk::game(SEED, &[headhunter()], &[], &[], &[forest(), island()]);
    let hunter = lk::obj(&e, lk::P0, headhunter());
    let fst = in_hand(&e, lk::P1, forest()).unwrap();

    let _ = lk::attack(&mut e, &[hunter]);
    lk::block(&mut e, &[]);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player, min, max, ..
    } = e.pending().clone()
    else {
        unreachable!();
    };
    assert_eq!(player, lk::P1);
    assert_eq!((min, max), (1, 1));
    e.apply(lk::P1, PlayerAction::ChooseObjects { objects: vec![fst] })
        .unwrap();
    lk::through_combat(&mut e);

    assert!(in_graveyard(&e, lk::P1, forest()).is_some());
    assert_eq!(cards_in(&e, ZoneLocation::Hand(lk::P1)), 1);

    // Blocked by a 2/2: deals damage only to the creature, so no discard.
    let mut e2 = lk::game(
        SEED,
        &[headhunter()],
        &[pearled_unicorn()],
        &[],
        &[island()],
    );
    let hunter2 = lk::obj(&e2, lk::P0, headhunter());
    let unicorn = lk::obj(&e2, lk::P1, pearled_unicorn());
    let _ = lk::attack(&mut e2, &[hunter2]);
    lk::block(&mut e2, &[(unicorn, hunter2)]);
    lk::through_combat(&mut e2);
    assert_eq!(cards_in(&e2, ZoneLocation::Hand(lk::P1)), 1);
}

#[test]
fn headhunter_casts_face_down_and_turns_face_up_for_black() {
    let mut e = lk::game(SEED, &[swamp(); 4], &[], &[headhunter()], &[]);
    cast_from_hand(&mut e, lk::P0, headhunter());
    let slot = choose_cast_kind(&e, CastModeKind::Disguise);
    e.apply(lk::P0, PlayerAction::ChooseMode(slot)).unwrap();
    lk::settle(&mut e);

    let id = lk::obj(&e, lk::P0, headhunter());
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
    .expect("remaining {B} pays Morph {B}");
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
