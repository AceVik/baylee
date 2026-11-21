//! `cards/creatures/mv_2/boneknitter.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn boneknitter() -> CardIndex {
    card_index("34b2f465-0b15-428a-a1c2-bcd41af07d01")
}

/// Boneknitter ({1}{B} 1/1 Zombie Cleric):
/// "{1}{B}: Regenerate target Zombie. Morph {2}{B}"
#[test]
fn boneknitter_regenerates_target_zombie_and_rejects_non_zombies() {
    let mut e = lk::game(
        SEED,
        &[boneknitter(), scathe_zombies(), grizzly_bears()],
        &[],
        &[lightning_bolt()],
        &[],
    );
    let knitter = lk::obj(&e, lk::P0, boneknitter());
    let zombie = lk::obj(&e, lk::P0, scathe_zombies());
    let bear = lk::obj(&e, lk::P0, grizzly_bears());

    lk::float(&mut e, lk::P0, &[(ManaColor::Black, 2)]);
    activate(&mut e, lk::P0, boneknitter(), 0);
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("expected Zombie target choice, got {:?}", e.pending());
    };
    assert!(options.contains(&knitter) && options.contains(&zombie));
    assert!(!options.contains(&bear), "non-Zombie cannot be targeted");
    lk::aim(&mut e, &[zombie], &[]);
    lk::settle(&mut e);

    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut e, lk::P0, lightning_bolt(), &[zombie], &[]);
    assert!(on_battlefield(&e, lk::P0, scathe_zombies()).is_some());
    assert!(lk::tapped(&e, zombie));
    assert_eq!(lk::damage_on(&e, zombie), 0);
}

#[test]
fn boneknitter_casts_face_down_and_turns_face_up_for_two_and_black() {
    let mut e = lk::game(SEED, &[swamp(); 6], &[], &[boneknitter()], &[]);
    cast_from_hand(&mut e, lk::P0, boneknitter());
    let slot = choose_cast_kind(&e, CastModeKind::Disguise);
    e.apply(lk::P0, PlayerAction::ChooseMode(slot)).unwrap();
    lk::settle(&mut e);

    let id = lk::obj(&e, lk::P0, boneknitter());
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
    .expect("remaining {2}{B} pays Morph {2}{B}");
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
