//! `cards/creatures/mv_1/foothill_guide.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn foothill_guide() -> CardIndex {
    card_index("d91c7e21-43af-40f9-b544-f39d1e77dee9")
}

/// Foothill Guide ({W} 1/1 Human Cleric):
/// "Protection from Goblins; Morph {W}"
#[test]
fn foothill_guide_has_protection_from_goblins_in_blocking_and_damage() {
    let mut e = lk::game(
        SEED,
        &[foothill_guide()],
        &[raging_goblin(), grizzly_bears()],
        &[],
        &[],
    );
    let guide = lk::obj(&e, lk::P0, foothill_guide());
    let goblin = lk::obj(&e, lk::P1, raging_goblin());
    let bear = lk::obj(&e, lk::P1, grizzly_bears());

    let blocks = lk::attack(&mut e, &[guide]);
    assert!(
        !lk::may_block(&blocks, goblin, guide),
        "Goblin cannot block Foothill Guide"
    );
    assert!(
        lk::may_block(&blocks, bear, guide),
        "non-Goblin can block Foothill Guide"
    );
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, lk::P1), 19);

    // Untapped Foothill Guide on next combat prevents Goblin combat damage.
    let mut e2 = lk::game(SEED, &[foothill_guide()], &[raging_goblin()], &[], &[]);
    let guide2 = lk::obj(&e2, lk::P0, foothill_guide());
    let goblin2 = lk::obj(&e2, lk::P1, raging_goblin());
    reach_their_main_phase(&mut e2, lk::P1);
    let opp_blocks = lk::attack(&mut e2, &[goblin2]);
    assert!(lk::may_block(&opp_blocks, guide2, goblin2));
    lk::block(&mut e2, &[(guide2, goblin2)]);
    lk::through_combat(&mut e2);
    assert!(on_battlefield(&e2, lk::P0, foothill_guide()).is_some());
    assert!(on_battlefield(&e2, lk::P1, raging_goblin()).is_none());
}

#[test]
fn foothill_guide_casts_face_down_without_protection_and_turns_face_up_for_white() {
    let mut e = lk::game(SEED, &[plains(); 4], &[], &[foothill_guide()], &[]);
    cast_from_hand(&mut e, lk::P0, foothill_guide());
    let slot = choose_cast_kind(&e, CastModeKind::Disguise);
    e.apply(lk::P0, PlayerAction::ChooseMode(slot)).unwrap();
    lk::settle(&mut e);

    let id = lk::obj(&e, lk::P0, foothill_guide());
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
    .expect("remaining {W} pays Morph {W}");
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
