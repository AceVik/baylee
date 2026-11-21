//! `cards/sorceries/mv_1/sylvan_tutor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);

fn sylvan_tutor() -> CardIndex {
    card_index("935e0cac-51ee-4cb7-a209-f085e0f099ed")
}

/// Sylvan Tutor (935e0cac-51ee-4cb7-a209-f085e0f099ed) — {G} Sorcery:
/// "Search your library for a creature card, reveal it, then shuffle and put
/// that card on top."
#[test]
fn sylvan_tutor_finds_only_creatures_reveals_shuffles_and_puts_on_top() {
    let mut engine = lk::game(
        SEED,
        &[forest()],
        &[],
        &[
            sylvan_tutor(),
            llanowar_elves(),
            aurochs(),
            counterspell(),
            sol_ring(),
        ],
        &[],
    );
    let elf = hand_to_library_top(&mut engine, P0, llanowar_elves());
    let beast = hand_to_library_top(&mut engine, P0, aurochs());
    let spell = hand_to_library_top(&mut engine, P0, counterspell());
    let ring = hand_to_library_top(&mut engine, P0, sol_ring());
    let lib_before = library_size(&engine, P0);

    cast_from_hand(&mut engine, P0, sylvan_tutor());
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
    assert_eq!(options.len(), 2, "only the two creature cards match");
    assert!(options.contains(&elf) && options.contains(&beast));
    assert!(!options.contains(&spell) && !options.contains(&ring));

    let before_entries = engine.journal().entries().len();
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![beast],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    let revealed = engine.journal().entries()[before_entries..].iter().any(
        |entry| matches!(&entry.event, GameEvent::Revealed { cards, .. } if cards.contains(&beast)),
    );
    assert!(revealed, "the chosen creature card is revealed");
    assert_eq!(library_size(&engine, P0), lib_before);
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(P0))
            .last()
            .copied(),
        Some(beast),
        "the chosen creature card sits on top of the shuffled library"
    );
}
