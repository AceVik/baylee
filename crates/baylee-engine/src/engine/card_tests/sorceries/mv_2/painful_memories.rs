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

#[test]
fn painful_memories_reveals_target_opponents_hand() {
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
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some());
    assert!(in_graveyard(&engine, P0, painful_memories()).is_some());
}
