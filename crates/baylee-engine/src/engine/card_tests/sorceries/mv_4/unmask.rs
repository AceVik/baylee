//! `cards/sorceries/mv_4/unmask.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "1a3a274d-41da-47db-ad41-5720f36a7963"
fn unmask() -> CardIndex {
    card_index("1a3a274d-41da-47db-ad41-5720f36a7963")
}

/// `Unmask`:
/// "You may exile a black card from your hand rather than pay this spell's mana cost."
/// "Target player reveals their hand. You choose a nonland card from it. That player discards that card."
#[test]
fn unmask_chooses_nonland_card_from_target_players_hand_to_discard() {
    let mut engine = lk::game(
        SEED,
        &[index::SWAMP, index::SWAMP, index::SWAMP, index::SWAMP],
        &[],
        &[unmask()],
        &[index::FOREST, index::GRIZZLY_BEARS],
    );
    let land = in_hand(&engine, P1, index::FOREST).expect("Forest in P1 hand");
    let bear = in_hand(&engine, P1, index::GRIZZLY_BEARS).expect("Grizzly Bears in P1 hand");

    cast_from_hand(&mut engine, P0, unmask());
    engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });

    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!(
            "Unmask should ask P0 to choose a card, got {:?}",
            engine.pending()
        );
    };
    assert!(
        options.contains(&bear),
        "Nonland card (Grizzly Bears) is offered by Unmask"
    );
    assert!(
        !options.contains(&land),
        "Land card (Forest) is not offered by Unmask"
    );

    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![bear],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some());
    assert!(in_hand(&engine, P1, index::FOREST).is_some());
}

#[test]
fn unmask_pitches_a_black_card_from_hand_without_mana() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[],
        &[unmask(), index::TERROR, index::GRIZZLY_BEARS],
        &[index::GRIZZLY_BEARS],
    );
    let unmask_id = in_hand(&engine, P0, unmask()).expect("Unmask in P0 hand");
    let black_card = in_hand(&engine, P0, index::TERROR).expect("Terror in P0 hand");
    let green_card = in_hand(&engine, P0, index::GRIZZLY_BEARS).expect("Grizzly Bears in P0 hand");
    let opp_bear = in_hand(&engine, P1, index::GRIZZLY_BEARS).expect("Grizzly Bears in P1 hand");

    engine
        .apply(P0, PlayerAction::CastSpell { card: unmask_id })
        .unwrap();
    engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();

    // Pitch cost: only black cards in P0's hand are offered.
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected pitch ChooseCards, got {:?}", engine.pending());
    };
    assert!(options.contains(&black_card));
    assert!(!options.contains(&green_card));
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![black_card],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![opp_bear],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(black_card).unwrap().zone, Zone::Exile);
    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some());
}
