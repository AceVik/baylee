//! `cards/sorceries/mv_4/eureka.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::partials_w2c_kit::{bears, hill_giant};

fn eureka() -> CardIndex {
    card_index("917f2b4b-79ec-4ded-b4c8-cbcf363cb23b")
}

/// Eureka: "Starting with you, each player may put a permanent card from their
/// hand onto the battlefield. Repeat this process until no one puts a card
/// onto the battlefield."
#[test]
fn eureka_alternates_players_putting_permanents_until_a_full_round_passes() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(42, lightning_bolt())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[eureka(), bears(), hill_giant(), lightning_bolt()])
        .hand(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches main phase");

    let bears_hand = in_hand(&engine, p0, bears()).expect("Bears in p0 hand");
    let giant_hand = in_hand(&engine, p0, hill_giant()).expect("Giant in p0 hand");
    let bolt_hand = in_hand(&engine, p0, lightning_bolt()).expect("Bolt in p0 hand");
    let elves_hand = in_hand(&engine, p1, llanowar_elves()).expect("Elves in p1 hand");

    cast_from_hand(&mut engine, p0, eureka());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });

    // Round 1, p0: offered permanent cards in p0's hand (Bears and Giant, not Bolt).
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseCards for p0, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (0, 1));
    assert!(options.contains(&bears_hand));
    assert!(options.contains(&giant_hand));
    assert!(
        !options.contains(&bolt_hand),
        "instant card is not a permanent card"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears_hand],
            },
        )
        .unwrap();

    // Round 1, p1: offered permanent cards in p1's hand (Llanowar Elves).
    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseCards for p1, got {:?}", engine.pending());
    };
    assert_eq!(player, p1);
    assert_eq!(options, vec![elves_hand]);
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![elves_hand],
            },
        )
        .unwrap();

    // Round 2, p0: Hill Giant is still in p0's hand; p0 puts Hill Giant onto
    // the battlefield.
    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected Round 2 ChooseCards for p0, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, p0);
    assert_eq!(options, vec![giant_hand]);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![giant_hand],
            },
        )
        .unwrap();

    // Neither player has any permanent cards left in hand, so Round 3 finishes
    // without prompting and Eureka completes resolution.
    assert!(
        stack_is_empty(&engine),
        "Eureka finished once a round produced no put"
    );
    assert!(on_battlefield(&engine, p0, bears()).is_some());
    assert!(on_battlefield(&engine, p0, hill_giant()).is_some());
    assert!(on_battlefield(&engine, p1, llanowar_elves()).is_some());
}
