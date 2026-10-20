//! `cards/sorceries/mv_2/diabolic_vision.rs`, played.
//!
//! Oracle:
//! - "Look at the top five cards of your library. Put one of them into your hand and the rest on top of your library in any order."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

// oracle_id = "6b7b1a71-812d-4ff5-b38b-94d1cf634143"
fn diabolic_vision() -> CardIndex {
    card_index("6b7b1a71-812d-4ff5-b38b-94d1cf634143")
}

#[test]
fn diabolic_vision_looks_at_top_five_and_puts_one_into_hand() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[],
        &[diabolic_vision(), index::LIGHTNING_BOLT],
        &[],
    );
    let lightning = in_hand(&engine, P0, index::LIGHTNING_BOLT).expect("in hand");
    let state = engine.dev_state_mut(P0).unwrap();
    state
        .move_object(
            lightning,
            ZoneLocation::Library(P0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .expect("move Lightning Bolt to top of library");
    engine.refresh_offer();

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Blue, 1), (ManaColor::Black, 1)],
    );
    cast_with_floating(&mut engine, P0, diabolic_vision());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCards for Diabolic Vision");
    };
    assert_eq!(
        options.len(),
        5,
        "looks at the top five cards of your library"
    );
    assert!(options.contains(&lightning));

    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![lightning],
            },
        )
        .expect("pick Lightning Bolt for hand");

    // Order the remaining cards if prompted.
    if let Pending::Arrange { cards, .. } = engine.pending().clone() {
        engine
            .apply(P0, PlayerAction::Arrange { piles: vec![cards] })
            .expect("order remaining four cards");
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_hand(&engine, P0, index::LIGHTNING_BOLT).is_some(),
        "chosen card is put into hand"
    );
}
