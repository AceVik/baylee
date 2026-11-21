//! `cards/creatures/mv_2/riptide_biologist.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn riptide_biologist() -> CardIndex {
    card_index("fe378f26-5c80-4548-8127-10724ac0c452")
}

/// Riptide Biologist ({1}{U} 1/2 Human Wizard):
/// "Protection from Beasts; Morph {2}{U}"
#[test]
fn riptide_biologist_has_protection_from_beasts_in_blocking_and_damage() {
    let mut e = lk::game(
        SEED,
        &[riptide_biologist()],
        &[index::BARKHIDE_MAULER, grizzly_bears()],
        &[],
        &[],
    );
    let bio = lk::obj(&e, lk::P0, riptide_biologist());
    let beast = lk::obj(&e, lk::P1, index::BARKHIDE_MAULER);
    let bear = lk::obj(&e, lk::P1, grizzly_bears());

    let blocks = lk::attack(&mut e, &[bio]);
    assert!(
        !lk::may_block(&blocks, beast, bio),
        "Beast cannot block Riptide Biologist"
    );
    assert!(
        lk::may_block(&blocks, bear, bio),
        "Bear (non-Beast) can block"
    );
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, lk::P1), 19);

    // Blocking a 4/4 Beast prevents all 4 combat damage to Riptide Biologist.
    let mut e2 = lk::game(
        SEED,
        &[riptide_biologist()],
        &[index::BARKHIDE_MAULER],
        &[],
        &[],
    );
    let bio2 = lk::obj(&e2, lk::P0, riptide_biologist());
    let beast2 = lk::obj(&e2, lk::P1, index::BARKHIDE_MAULER);
    reach_their_main_phase(&mut e2, lk::P1);
    let _ = lk::attack(&mut e2, &[beast2]);
    lk::block(&mut e2, &[(bio2, beast2)]);
    lk::through_combat(&mut e2);
    assert!(on_battlefield(&e2, lk::P0, riptide_biologist()).is_some());
    assert_eq!(lk::damage_on(&e2, bio2), 0);
}

#[test]
fn riptide_biologist_casts_face_down_and_turns_face_up_for_two_and_blue() {
    let mut e = lk::game(SEED, &[island(); 6], &[], &[riptide_biologist()], &[]);
    cast_from_hand(&mut e, lk::P0, riptide_biologist());
    let slot = choose_cast_kind(&e, CastModeKind::Disguise);
    e.apply(lk::P0, PlayerAction::ChooseMode(slot)).unwrap();
    lk::settle(&mut e);

    let id = lk::obj(&e, lk::P0, riptide_biologist());
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
    .expect("remaining {2}{U} pays Morph {2}{U}");
    assert!(
        !e.state()
            .object(id)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );
    assert_eq!(pt(&e, id), (1, 2));
    assert_eq!(lk::pool_total(&e, lk::P0), 0);
}
