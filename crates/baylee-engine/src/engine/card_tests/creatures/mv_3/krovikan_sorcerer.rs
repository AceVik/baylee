//! `cards/creatures/mv_3/krovikan_sorcerer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn krovikan_sorcerer() -> CardIndex {
    card_index("00e6808c-1f9e-4b6a-807c-83f433b81a41")
}

/// Krovikan Sorcerer (00e6808c-1f9e-4b6a-807c-83f433b81a41) — {2}{U} 1/1 Creature — Human Wizard Sorcerer:
/// - "{T}, Discard a nonblack card: Draw a card."
/// - "{T}, Discard a black card: Draw two cards, then discard one of them."
#[test]
fn krovikan_sorcerer_first_ability_discards_only_nonblack_card_and_draws_one() {
    let mut engine = lk::game(
        SEED,
        &[krovikan_sorcerer()],
        &[],
        &[index::GRIZZLY_BEARS, index::DRUDGE_SKELETONS],
        &[],
    );
    let sorcerer = lk::obj(&engine, P0, krovikan_sorcerer());
    let bears_in_hand = in_hand(&engine, P0, index::GRIZZLY_BEARS).expect("Bears in hand");
    let skel_in_hand = in_hand(&engine, P0, index::DRUDGE_SKELETONS).expect("Skeletons in hand");

    activate(&mut engine, P0, krovikan_sorcerer(), 0);
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCards for discard cost");
    };
    assert!(
        options.contains(&bears_in_hand),
        "nonblack Grizzly Bears is offered for 'Discard a nonblack card'"
    );
    assert!(
        !options.contains(&skel_in_hand),
        "black Drudge Skeletons is excluded from 'Discard a nonblack card'"
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
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        2,
        "discarded 1 nonblack card and drew 1 card"
    );
}

#[test]
fn krovikan_sorcerer_second_ability_discards_only_black_card_draws_two_then_discards_one() {
    let mut engine = lk::game(
        SEED,
        &[krovikan_sorcerer()],
        &[],
        &[index::GRIZZLY_BEARS, index::DRUDGE_SKELETONS],
        &[],
    );
    let bears_in_hand = in_hand(&engine, P0, index::GRIZZLY_BEARS).expect("Bears in hand");
    let skel_in_hand = in_hand(&engine, P0, index::DRUDGE_SKELETONS).expect("Skeletons in hand");

    activate(&mut engine, P0, krovikan_sorcerer(), 1);
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCards for discard cost");
    };
    assert!(
        options.contains(&skel_in_hand),
        "black Drudge Skeletons is offered for 'Discard a black card'"
    );
    assert!(
        !options.contains(&bears_in_hand),
        "nonblack Grizzly Bears is excluded from 'Discard a black card'"
    );

    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![skel_in_hand],
            },
        )
        .unwrap();

    // Resolution draws 2 cards, then asks P0 to discard 1 card.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let hand = engine.state().zones.list(ZoneLocation::Hand(P0)).clone();
    assert_eq!(
        hand.len(),
        3,
        "after discarding Drudge Skeletons (down to 1) and drawing 2 cards, P0 has 3 cards before the resolution discard"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![hand[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        2,
        "after discarding 1 card on resolution, P0 ends with 2 cards in hand"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(P0)).len(),
        2,
        "both the black cost card and the resolution discard are in P0's graveyard"
    );
}
