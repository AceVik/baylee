//! `cards/sorceries/mv_2/foresight.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn foresight() -> CardIndex {
    card_index("c9640b04-9751-46d5-b7bf-5b8a8ecf145f")
}

#[test]
fn foresight_shuffles_library_and_draws_on_next_turns_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island()])
        .hand(0, &[foresight()])
        .start();

    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    cast_from_hand(&mut engine, p0, foresight());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1
    );

    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.step == Step::Upkeep
    });
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "Foresight should draw a card at the beginning of the next turn's upkeep"
    );
}

/// Known gap, pinned: "Search your library for three cards, exile them" is not on the card, so no
/// question is asked, nothing leaves the library but the one card drawn next upkeep, and the
/// shuffle and the draw (once, the caster's) are all it does. When the DSL can search into exile,
/// this test must move to the card that asks for three cards.
#[test]
fn foresight_exiles_nothing_and_draws_exactly_one_card_next_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island()])
        .hand(0, &[foresight()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let library =
        |e: &Engine<RegistryLookup>| e.state().zones.list(ZoneLocation::Library(p0)).len();
    let hand = |e: &Engine<RegistryLookup>| e.state().zones.list(ZoneLocation::Hand(p0)).len();
    let (library_before, hand_before) = (library(&engine), hand(&engine));

    cast_from_hand(&mut engine, p0, foresight());
    // A search would stop this walk at its question.
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        library(&engine),
        library_before,
        "no card was searched out of the library"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.step == Step::Upkeep
    });
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(library(&engine), library_before - 1);
    assert_eq!(hand(&engine), hand_before, "one card in, the spell out");

    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        hand(&engine),
        hand_before + 1,
        "the turn after brings only the ordinary draw"
    );
}
