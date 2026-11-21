//! `cards/sorceries/mv_3/cruel_tutor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn cruel_tutor() -> CardIndex {
    card_index("d9353eb6-c7e7-4d85-a0c2-32b1d091881a")
}

/// Cruel Tutor (d9353eb6-c7e7-4d85-a0c2-32b1d091881a) — {2}{B} Sorcery:
/// "Search your library for a card, then shuffle and put that card on top. You
/// lose 2 life."
#[test]
fn cruel_tutor_searches_any_card_puts_it_on_top_and_loses_two_life() {
    let mut engine = lk::game(
        SEED,
        &[swamp(), island(), island()],
        &[],
        &[cruel_tutor(), sol_ring(), counterspell(), llanowar_elves()],
        &[],
    );
    let ring = hand_to_library_top(&mut engine, P0, sol_ring());
    let spell = hand_to_library_top(&mut engine, P0, counterspell());
    let elf = hand_to_library_top(&mut engine, P0, llanowar_elves());
    let lib_before = library_size(&engine, P0);

    cast_from_hand(&mut engine, P0, cruel_tutor());
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
    assert_eq!(options.len(), lib_before, "every card in library is legal");
    assert!(options.contains(&ring) && options.contains(&spell) && options.contains(&elf));

    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![ring],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    assert_eq!(library_size(&engine, P0), lib_before);
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(P0))
            .last()
            .copied(),
        Some(ring),
        "chosen card placed on top of library"
    );
    assert_eq!(lk::life(&engine, P0), 18, "caster loses 2 life");
    assert_eq!(lk::life(&engine, P1), 20);
}
