//! `cards/sorceries/mv_2/painful_memories.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "27a3faa1-4f1a-4581-9e19-9265faa9c10b"
fn painful_memories() -> CardIndex {
    card_index("27a3faa1-4f1a-4581-9e19-9265faa9c10b")
}

/// Only the look is there: the caster sees the opponent's hand, and no card
/// leaves it. (The old card made the opponent discard the chosen card, which
/// loses it for good, where the printed spell puts it on top of the library.)
#[test]
fn painful_memories_only_looks_at_the_opponents_hand_and_moves_nothing() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[],
        &[painful_memories()],
        &[index::GRIZZLY_BEARS],
    );
    let bears = in_hand(&engine, P1, index::GRIZZLY_BEARS).unwrap();

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::Black, 1)],
    );
    cast_with_floating(&mut engine, P0, painful_memories());
    engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!();
    };
    assert_eq!(player, P0, "the caster is the one who looks");
    assert!(options.contains(&bears), "the opponent's hand is shown");
    engine
        .apply(P0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    lk::settle(&mut engine);

    assert!(
        in_hand(&engine, P1, index::GRIZZLY_BEARS).is_some(),
        "no card is discarded"
    );
    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_none());
    assert!(in_graveyard(&engine, P0, painful_memories()).is_some());
}
