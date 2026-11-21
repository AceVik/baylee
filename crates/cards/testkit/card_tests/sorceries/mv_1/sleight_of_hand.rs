//! `cards/sorceries/mv_1/sleight_of_hand.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);

fn sleight_of_hand() -> CardIndex {
    card_index("05a039ad-1689-4d13-b393-6c1219583b5d")
}

/// Sleight of Hand (05a039ad-1689-4d13-b393-6c1219583b5d) — {U} Sorcery:
/// "Look at the top two cards of your library. Put one of them into your hand
/// and the other on the bottom of your library."
#[test]
fn sleight_of_hand_puts_one_of_top_two_into_hand_and_other_on_bottom() {
    let mut engine = lk::game(
        SEED,
        &[island()],
        &[],
        &[
            sleight_of_hand(),
            aurochs(),
            llanowar_elves(),
            counterspell(),
        ],
        &[],
    );
    let third = hand_to_library_top(&mut engine, P0, aurochs());
    let second = hand_to_library_top(&mut engine, P0, llanowar_elves());
    let top = hand_to_library_top(&mut engine, P0, counterspell());

    cast_from_hand(&mut engine, P0, sleight_of_hand());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!();
    };
    assert_eq!(player, P0);
    assert_eq!((min, max), (1, 1));
    assert_eq!(options.len(), 2, "only the top two cards are looked at");
    assert!(options.contains(&top) && options.contains(&second));
    assert!(!options.contains(&third));

    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![second],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(P0))
            .contains(&second),
        "chosen card moved to hand"
    );
    let library = engine.state().zones.list(ZoneLocation::Library(P0));
    assert_eq!(
        library.first().copied(),
        Some(top),
        "the unchosen card moved to the bottom of the library"
    );
    assert_eq!(
        library.last().copied(),
        Some(third),
        "the third card is now on top of the library"
    );
}
