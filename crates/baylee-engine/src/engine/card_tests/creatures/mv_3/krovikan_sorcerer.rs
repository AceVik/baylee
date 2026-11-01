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
///   (Partial: the second ability is left off, because no effect discards one of the cards drawn
///   this way and a discard of any hand card is a different choice.)
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

    // A black card is in hand, so the printed second ability would be on offer; it is not
    // implemented, and no variant that discards a card from before the draw stands in for it.
    assert!(
        !ability_offered(&engine, sorcerer, 1),
        "the second ability is not on the card, so it is not offered with a black card in hand"
    );

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
