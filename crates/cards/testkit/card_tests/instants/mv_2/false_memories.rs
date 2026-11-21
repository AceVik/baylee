//! `cards/instants/mv_2/false_memories.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "d95dd8cc-ce7d-4d43-afbd-efaa2d3c7a35"
fn false_memories() -> CardIndex {
    card_index("d95dd8cc-ce7d-4d43-afbd-efaa2d3c7a35")
}

/// False Memories ({1}{U}): mills 7 cards immediately, then at the beginning
/// of the next end step exiles 7 chosen cards from your graveyard (one-shot,
/// does not repeat on the following end step).
#[test]
fn false_memories_mills_seven_and_exiles_seven_at_next_end_step() {
    let mut engine = lk::game(SEED, &[], &[], &[false_memories()], &[]);
    let lib_before = engine.state().zones.list(ZoneLocation::Library(P0)).len();

    lk::float(&mut engine, P0, &[(ManaColor::Blue, 2)]);
    cast_with_floating(&mut engine, P0, false_memories());
    lk::settle(&mut engine);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(P0)).len(),
        lib_before - 7,
        "milled seven cards"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(P0)).len(),
        8,
        "7 milled cards + False Memories itself in graveyard"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(P0))
            .is_empty(),
        "nothing exiled until the end step"
    );

    // Advance to the end step delayed trigger's card-choice prompt.
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
        panic!(
            "expected ChooseCards at end step, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, P0);
    assert_eq!(
        (min, max),
        (7, 7),
        "exile exactly seven cards from graveyard"
    );
    assert_eq!(options.len(), 8);
    let to_exile: Vec<ObjectId> = options.into_iter().take(7).collect();
    engine
        .apply(P0, PlayerAction::ChooseObjects { objects: to_exile })
        .unwrap();
    lk::settle(&mut engine);

    assert_eq!(engine.state().zones.list(ZoneLocation::Exile(P0)).len(), 7);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(P0)).len(),
        1
    );

    // Next turn passes cleanly without firing the one-shot delayed trigger again.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    assert_eq!(engine.state().zones.list(ZoneLocation::Exile(P0)).len(), 7);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(P0)).len(),
        1
    );
}
