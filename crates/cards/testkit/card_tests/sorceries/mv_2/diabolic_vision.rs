//! `cards/sorceries/mv_2/diabolic_vision.rs`, played.
//!
//! Oracle:
//! - "Look at the top five cards of your library. Put one of them into your hand and the rest on top of your library in any order."

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::{legends_kit as lk, partials_w1b_kit as kit};
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn diabolic_vision() -> CardIndex {
    card_index("6b7b1a71-812d-4ff5-b38b-94d1cf634143")
}

struct Cast {
    engine: Engine<RegistryLookup>,
    stacked: Vec<ObjectId>,
    spell: ObjectId,
}

/// Casts Vision with a library containing exactly `top_first` in that order.
fn cast(top_first: &[CardIndex]) -> Cast {
    let vision = diabolic_vision();
    let mut hand = vec![vision];
    hand.extend_from_slice(top_first);
    let mut engine = Duel::new(SEED, index::SWAMP).hand(0, &hand).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let spell = in_hand(&engine, P0, vision).expect("Vision starts in hand");
    let stacked = kit::replace_library(&mut engine, P0, top_first);
    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Blue, 1), (ManaColor::Black, 1)],
    );
    cast_with_floating(&mut engine, P0, vision);
    Cast {
        engine,
        stacked,
        spell,
    }
}

fn hand(engine: &Engine<RegistryLookup>) -> Vec<ObjectId> {
    engine.state().zones.list(ZoneLocation::Hand(P0)).clone()
}

#[track_caller]
fn assert_private_look_and_no_draw(engine: &Engine<RegistryLookup>) {
    assert!(
        !engine
            .state()
            .journal
            .entries()
            .iter()
            .any(|entry| matches!(entry.event, GameEvent::Revealed { .. })),
        "looking at a library does not reveal it"
    );
    assert!(
        !engine
            .state()
            .journal
            .entries()
            .iter()
            .any(|entry| matches!(entry.event, GameEvent::CardsDrawn { .. })),
        "putting the chosen card into hand is not drawing it"
    );
}

#[track_caller]
fn assert_only_vision_in_graveyard(cast: &Cast) {
    assert_eq!(kit::graveyard_of(&cast.engine, P0), vec![cast.spell]);
}

#[test]
fn diabolic_vision_puts_one_in_hand_and_orders_the_rest_on_top() {
    let mut cast = cast(&[
        index::SWAMP,
        index::FOREST,
        index::LIGHTNING_BOLT,
        index::MOUNTAIN,
        index::PLAINS,
        index::ISLAND,
        index::SWAMP,
    ]);
    let opponent_library = kit::library_of(&cast.engine, P1);
    pass_until(&mut cast.engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = cast.engine.pending().clone()
    else {
        panic!("Vision offers a private card choice");
    };
    assert_eq!(player, P0);
    assert_eq!(options, cast.stacked[..5]);
    assert_eq!((min, max), (1, 1));
    assert_eq!(prompt, ChoicePrompt::PutIntoHand);

    let chosen = cast.stacked[2];
    cast.engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .expect("choose one of the inspected cards");
    let Pending::Arrange {
        player,
        cards,
        piles,
        ..
    } = cast.engine.pending().clone()
    else {
        panic!("the other four return to the top in an order");
    };
    assert_eq!(player, P0);
    assert_eq!(
        cards,
        vec![
            cast.stacked[0],
            cast.stacked[1],
            cast.stacked[3],
            cast.stacked[4],
        ]
    );
    assert_eq!(piles.len(), 1);
    assert_eq!(piles[0].place, ArrangePlace::LibraryTop);

    let order = vec![
        cast.stacked[4],
        cast.stacked[0],
        cast.stacked[3],
        cast.stacked[1],
    ];
    cast.engine
        .apply(
            P0,
            PlayerAction::Arrange {
                piles: vec![order.clone()],
            },
        )
        .expect("choose any order for the remaining four");
    pass_until(&mut cast.engine, stack_is_empty);

    assert_eq!(hand(&cast.engine), vec![chosen]);
    let mut expected_library = vec![cast.stacked[6], cast.stacked[5]];
    expected_library.extend(order.iter().rev());
    assert_eq!(
        kit::library_of(&cast.engine, P0),
        expected_library,
        "the uninspected suffix stays below the four reordered cards"
    );
    assert_eq!(kit::library_of(&cast.engine, P1), opponent_library);
    assert_only_vision_in_graveyard(&cast);
    assert_private_look_and_no_draw(&cast.engine);
}

#[test]
fn diabolic_vision_uses_every_available_card_from_a_short_library() {
    for size in [1, 3] {
        let mut cast = cast(&vec![index::SWAMP; size]);
        pass_until(&mut cast.engine, |e| {
            matches!(e.pending(), Pending::ChooseCards { .. })
        });
        let Pending::ChooseCards {
            options, min, max, ..
        } = cast.engine.pending().clone()
        else {
            panic!("a nonempty short library still offers its cards");
        };
        assert_eq!(options, cast.stacked);
        assert_eq!((min, max), (1, 1));

        let chosen = cast.stacked[0];
        cast.engine
            .apply(
                P0,
                PlayerAction::ChooseObjects {
                    objects: vec![chosen],
                },
            )
            .expect("the one required pick is available");
        if let Pending::Arrange { cards, .. } = cast.engine.pending().clone() {
            cast.engine
                .apply(P0, PlayerAction::Arrange { piles: vec![cards] })
                .expect("the remaining short-library cards keep their order");
        }
        pass_until(&mut cast.engine, stack_is_empty);

        assert_eq!(hand(&cast.engine), vec![chosen]);
        assert_eq!(kit::library_of(&cast.engine, P0).len(), size - 1);
        assert_only_vision_in_graveyard(&cast);
        assert_private_look_and_no_draw(&cast.engine);
    }
}

#[test]
fn diabolic_vision_with_an_empty_library_asks_nothing() {
    let mut cast = cast(&[]);
    pass_until(&mut cast.engine, stack_is_empty);

    assert!(hand(&cast.engine).is_empty());
    assert!(kit::library_of(&cast.engine, P0).is_empty());
    assert!(!matches!(
        cast.engine.pending(),
        Pending::ChooseCards { .. }
    ));
    assert_only_vision_in_graveyard(&cast);
    assert_private_look_and_no_draw(&cast.engine);
}
