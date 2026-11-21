//! `cards/sorceries/mv_1/personal_tutor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);

fn personal_tutor() -> CardIndex {
    card_index("90f54959-2c9b-4b8a-84c9-d6893eb43553")
}

fn salvage() -> CardIndex {
    card_index("c01a090a-11e4-469e-aded-74203c06fef4")
}

fn chorus_of_woe() -> CardIndex {
    card_index("e175a3c1-df6c-4aa7-a4c9-893b466ad9e0")
}

/// Personal Tutor (90f54959-2c9b-4b8a-84c9-d6893eb43553) — {U} Sorcery:
/// "Search your library for a sorcery card, reveal it, then shuffle and put
/// that card on top."
#[test]
fn personal_tutor_finds_only_sorceries_reveals_shuffles_and_puts_on_top() {
    let mut engine = lk::game(
        SEED,
        &[island()],
        &[],
        &[
            personal_tutor(),
            salvage(),
            chorus_of_woe(),
            counterspell(),
            llanowar_elves(),
        ],
        &[],
    );
    let seal = hand_to_library_top(&mut engine, P0, salvage());
    let div = hand_to_library_top(&mut engine, P0, chorus_of_woe());
    let counter = hand_to_library_top(&mut engine, P0, counterspell());
    let elf = hand_to_library_top(&mut engine, P0, llanowar_elves());
    let lib_before = library_size(&engine, P0);

    cast_from_hand(&mut engine, P0, personal_tutor());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        unreachable!();
    };
    assert_eq!(player, P0);
    assert_eq!(prompt, ChoicePrompt::SearchLibrary);
    assert_eq!(options.len(), 2, "only the two sorcery cards match");
    assert!(options.contains(&seal) && options.contains(&div));
    assert!(!options.contains(&counter) && !options.contains(&elf));

    let before_entries = engine.journal().entries().len();
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![seal],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    let revealed = engine.journal().entries()[before_entries..].iter().any(
        |entry| matches!(&entry.event, GameEvent::Revealed { cards, .. } if cards.contains(&seal)),
    );
    assert!(revealed, "the chosen sorcery card is revealed");
    assert_eq!(library_size(&engine, P0), lib_before);
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(P0))
            .last()
            .copied(),
        Some(seal),
        "the chosen sorcery card sits on top of the shuffled library"
    );
}
