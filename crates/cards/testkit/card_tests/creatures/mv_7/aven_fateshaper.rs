//! `cards/creatures/mv_7/aven_fateshaper.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn aven_fateshaper() -> CardIndex {
    card_index("739d93c4-8d7b-4a54-bb6e-1d283e333392")
}

/// The top `n` cards of seat 0's library, top first.
fn top_cards(engine: &Engine<RegistryLookup>, n: usize) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Library(PlayerId::new(0)))
        .iter()
        .rev()
        .take(n)
        .copied()
        .collect()
}

/// Answers the reordering question with the looked-at cards in reverse order,
/// passing priority until it is asked; returns what was asked.
#[track_caller]
fn reverse_the_top_four(engine: &mut Engine<RegistryLookup>) -> Vec<ObjectId> {
    let p0 = PlayerId::new(0);
    pass_until(engine, |e| matches!(e.pending(), Pending::Arrange { .. }));
    let Pending::Arrange { cards, piles, .. } = engine.pending().clone() else {
        unreachable!("the pass stopped here")
    };
    assert_eq!(cards.len(), 4, "the top four cards are looked at");
    assert_eq!(piles.len(), 1, "one pile: they go back on top");
    let mut reversed = cards.clone();
    reversed.reverse();
    engine
        .apply(
            p0,
            PlayerAction::Arrange {
                piles: vec![reversed],
            },
        )
        .expect("any order of the four is legal");
    cards
}

/// Aven Fateshaper — a 4/5 flier for {6}{U}. Its enters trigger looks at the
/// top four cards and puts them back in any order: answered in reverse, the
/// library's top four are the same cards the other way round, and no card
/// left the library.
#[test]
fn aven_fateshaper_reorders_the_top_four_when_it_enters() {
    let p0 = PlayerId::new(0);
    let mut engine = table7(&lots(island(), 7), &[aven_fateshaper()], &[]);
    let before = library_size(&engine, p0);
    pay_exactly_and_cast(&mut engine, aven_fateshaper());
    let seen = reverse_the_top_four(&mut engine);
    pass_until(&mut engine, stack_is_empty);

    let bird = on_battlefield(&engine, p0, aven_fateshaper()).expect("resolved");
    assert_body(&engine, bird, (4, 5), KeywordSet::FLYING);
    assert_eq!(library_size(&engine, p0), before, "nothing was drawn");
    let mut expected = seen;
    expected.reverse();
    assert_eq!(top_cards(&engine, 4), expected, "the answer is the order");
}

/// {4}{U}: the same look and reorder, on demand.
#[test]
fn aven_fateshaper_reorders_again_for_four_and_blue() {
    let p0 = PlayerId::new(0);
    let mut board = lots(island(), 5);
    board.push(aven_fateshaper());
    let mut engine = table7(&board, &[], &[]);
    let before = library_size(&engine, p0);
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, aven_fateshaper(), 0);
    let seen = reverse_the_top_four(&mut engine);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(library_size(&engine, p0), before);
    let mut expected = seen;
    expected.reverse();
    assert_eq!(top_cards(&engine, 4), expected);
}
