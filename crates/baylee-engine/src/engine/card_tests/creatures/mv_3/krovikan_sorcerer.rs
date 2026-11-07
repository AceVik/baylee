//! `cards/creatures/mv_3/krovikan_sorcerer.rs`, played.
//!
//! Oracle:
//! - "{T}, Discard a nonblack card: Draw a card."
//! - "{T}, Discard a black card: Draw two cards, then discard one of them."

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::{legends_kit as lk, partials_w1b_kit as kit};
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn krovikan_sorcerer() -> CardIndex {
    card_index("00e6808c-1f9e-4b6a-807c-83f433b81a41")
}

#[test]
fn first_ability_discards_only_a_nonblack_card_and_draws_exactly_one() {
    let mut engine = lk::game(
        SEED,
        &[krovikan_sorcerer()],
        &[],
        &[
            index::GRIZZLY_BEARS,
            index::DRUDGE_SKELETONS,
            index::LIGHTNING_BOLT,
            index::ISLAND,
        ],
        &[],
    );
    let sorcerer = lk::obj(&engine, P0, krovikan_sorcerer());
    let bears_in_hand = in_hand(&engine, P0, index::GRIZZLY_BEARS).expect("Bears in hand");
    let skel_in_hand = in_hand(&engine, P0, index::DRUDGE_SKELETONS).expect("Skeletons in hand");
    let stacked = kit::replace_library(&mut engine, P0, &[index::LIGHTNING_BOLT, index::ISLAND]);

    assert!(ability_offered(&engine, sorcerer, 0));
    assert!(
        ability_offered(&engine, sorcerer, 1),
        "the black card in hand pays for the second ability"
    );

    activate(&mut engine, P0, krovikan_sorcerer(), 0);
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCards for discard cost");
    };
    assert_eq!(options, vec![bears_in_hand]);
    assert!(
        !options.contains(&skel_in_hand),
        "a black card cannot pay the nonblack discard cost"
    );

    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![bears_in_hand],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, sorcerer));
    assert!(in_graveyard(&engine, P0, index::GRIZZLY_BEARS).is_some());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).as_slice(),
        &[skel_in_hand, stacked[0]],
        "the black card remains and exactly the top card was drawn"
    );
}

#[test]
fn second_ability_discards_black_then_draws_two_and_discards_only_one_of_them() {
    let mut engine = lk::game(
        SEED,
        &[krovikan_sorcerer()],
        &[],
        &[
            index::GRIZZLY_BEARS,
            index::DRUDGE_SKELETONS,
            index::LIGHTNING_BOLT,
            index::ISLAND,
        ],
        &[],
    );
    let sorcerer = lk::obj(&engine, P0, krovikan_sorcerer());
    let bears_in_hand = in_hand(&engine, P0, index::GRIZZLY_BEARS).expect("Bears in hand");
    let skel_in_hand = in_hand(&engine, P0, index::DRUDGE_SKELETONS).expect("Skeletons in hand");
    let stacked = kit::replace_library(&mut engine, P0, &[index::LIGHTNING_BOLT, index::ISLAND]);

    assert!(ability_offered(&engine, sorcerer, 1));
    activate(&mut engine, P0, krovikan_sorcerer(), 1);
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCards for the black discard cost");
    };
    assert_eq!(options, vec![skel_in_hand]);
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ChooseObjects {
                    objects: vec![bears_in_hand],
                },
            )
            .is_err(),
        "a nonblack card cannot pay the black discard cost"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![skel_in_hand],
            },
        )
        .unwrap();
    pass_until(&mut engine, |engine| {
        matches!(engine.pending(), Pending::ChooseCards { .. })
    });

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!(player, P0);
    assert_eq!((min, max), (1, 1));
    assert_eq!(
        options, stacked,
        "only the two cards drawn by this ability are eligible"
    );
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ChooseObjects {
                    objects: vec![bears_in_hand],
                },
            )
            .is_err(),
        "a card held before the draw cannot be discarded as one of them"
    );

    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![stacked[1]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, sorcerer));
    assert!(in_graveyard(&engine, P0, index::DRUDGE_SKELETONS).is_some());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).as_slice(),
        &[bears_in_hand, stacked[0]],
        "the old card and the unchosen drawn card remain in hand"
    );
    assert_eq!(
        kit::graveyard_of(&engine, P0),
        vec![skel_in_hand, stacked[1]],
        "the black cost card and chosen drawn card were discarded"
    );
}
