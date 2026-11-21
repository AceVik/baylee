//! `cards/creatures/mv_3/nosy_goblin.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn nosy_goblin() -> CardIndex {
    card_index("7bca544e-08d8-4e83-aa30-c7201a649a66")
}

/// Nosy Goblin ({2}{R} 2/1 Goblin):
/// "{T}, Sacrifice this creature: Destroy target face-down creature."
#[test]
fn nosy_goblin_sacrifices_to_destroy_only_face_down_creatures() {
    let mut e = lk::game(
        SEED,
        &[nosy_goblin()],
        &[grizzly_bears(), swamp(), swamp(), swamp()],
        &[],
        &[index::SPINED_BASHER],
    );
    let bear = lk::obj(&e, lk::P1, grizzly_bears());
    assert!(
        !lk::offered(&e, nosy_goblin(), lk::P0, 0),
        "no face-down creature on the battlefield -> no legal target"
    );

    reach_their_main_phase(&mut e, lk::P1);
    cast_from_hand(&mut e, lk::P1, index::SPINED_BASHER);
    let slot = choose_cast_kind(&e, CastModeKind::Disguise);
    e.apply(lk::P1, PlayerAction::ChooseMode(slot)).unwrap();
    lk::settle(&mut e);

    let morph_id = lk::obj(&e, lk::P1, index::SPINED_BASHER);
    assert!(
        e.state()
            .object(morph_id)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );

    e.apply(lk::P1, PlayerAction::PassPriority).unwrap();
    activate(&mut e, lk::P0, nosy_goblin(), 0);
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("expected face-down target choice, got {:?}", e.pending());
    };
    assert_eq!(
        options,
        vec![morph_id],
        "face-up Bear is not a legal target"
    );
    assert!(!options.contains(&bear));
    lk::aim(&mut e, &[morph_id], &[]);
    lk::settle(&mut e);

    assert!(on_battlefield(&e, lk::P0, nosy_goblin()).is_none());
    assert!(in_graveyard(&e, lk::P0, nosy_goblin()).is_some());
    assert!(on_battlefield(&e, lk::P1, index::SPINED_BASHER).is_none());
    assert!(in_graveyard(&e, lk::P1, index::SPINED_BASHER).is_some());
}
