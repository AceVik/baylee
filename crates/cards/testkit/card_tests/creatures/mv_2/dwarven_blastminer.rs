//! `cards/creatures/mv_2/dwarven_blastminer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn dwarven_blastminer() -> CardIndex {
    card_index("b373e978-c58b-465a-b68a-9d4acec9dce0")
}

/// Dwarven Blastminer ({1}{R} 1/1 Dwarf):
/// "{2}{R}, {T}: Destroy target nonbasic land. Morph {R}"
#[test]
fn dwarven_blastminer_destroys_nonbasic_land_and_cannot_target_basic_lands() {
    let mut e = lk::game(
        SEED,
        &[dwarven_blastminer(), mountain()],
        &[badlands(), forest()],
        &[],
        &[],
    );
    let miner = lk::obj(&e, lk::P0, dwarven_blastminer());
    let mtn = lk::obj(&e, lk::P0, mountain());
    let bad = lk::obj(&e, lk::P1, badlands());
    let fst = lk::obj(&e, lk::P1, forest());

    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 3)]);
    activate(&mut e, lk::P0, dwarven_blastminer(), 0);
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!(
            "expected nonbasic land target choice, got {:?}",
            e.pending()
        );
    };
    assert_eq!(
        options,
        vec![bad],
        "basic Mountain and Forest are not legal targets"
    );
    assert!(!options.contains(&mtn) && !options.contains(&fst));
    lk::aim(&mut e, &[bad], &[]);
    lk::settle(&mut e);

    assert!(lk::tapped(&e, miner));
    assert!(on_battlefield(&e, lk::P1, badlands()).is_none());
    assert!(in_graveyard(&e, lk::P1, badlands()).is_some());
}

#[test]
fn dwarven_blastminer_casts_face_down_and_turns_face_up_for_red() {
    let mut e = lk::game(SEED, &[mountain(); 4], &[], &[dwarven_blastminer()], &[]);
    cast_from_hand(&mut e, lk::P0, dwarven_blastminer());
    let slot = choose_cast_kind(&e, CastModeKind::Disguise);
    e.apply(lk::P0, PlayerAction::ChooseMode(slot)).unwrap();
    lk::settle(&mut e);

    let id = lk::obj(&e, lk::P0, dwarven_blastminer());
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
