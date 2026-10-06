//! `cards/artifacts/mv_2/treasure_map.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Treasure Map` // `Treasure Cove` (`Coverage::Partial`): "{1}, {T}: Scry 1. Put a landmark
/// counter on this artifact. Then if there are three or more landmark counters on it, remove
/// those counters, transform this artifact, and create three Treasure tokens. // {T}: Add {C}.
/// {T}, Sacrifice a Treasure: Draw a card."
///
/// Under `Coverage::Partial`, the counter-count branch and the transform are omitted: no
/// effect removes counters, and nothing transforms a permanent in place (#206). The front-face
/// `{1}, {T}: Scry 1. Put a landmark counter on this artifact.` is written. The test activates
/// ability 0, answers the scry arrangement (`ArrangePrompt::Scry`) to bottom the top card, and
/// verifies the bottomed card, the tapped state, the landmark counter, and that `Treasure Map`
/// remains on face 0.
#[test]
fn treasure_map_activates_to_scry_one_and_put_a_landmark_counter() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(267, forest())
        .battlefield(0, &[forest(), forest(), treasure_map()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let map = on_battlefield(&engine, p0, treasure_map()).expect("map on battlefield");
    assert!(!is_tapped(&engine, map), "map starts untapped");

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, treasure_map(), 0);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });

    let Pending::Arrange {
        player,
        cards,
        piles,
        prompt,
    } = engine.pending().clone()
    else {
        unreachable!("pass_until only stops on card choice")
    };
    assert_eq!(player, p0, "the map's controller scries");
    assert_eq!(prompt, crate::choice::ArrangePrompt::Scry, "prompt is Scry");
    assert_eq!(piles, scry_piles(1), "Scry 1 allows bottoming 0 or 1 cards");
    assert_eq!(cards.len(), 1, "top card of library is inspected");
    let top_card = cards[0];

    engine.apply(p0, look_answer(&cards, &[top_card])).unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(
        is_tapped(&engine, map),
        "map tapped to activate its ability"
    );
    let library = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Library(p0))
        .clone();
    assert_eq!(
        library.first().copied(),
        Some(top_card),
        "the inspected card was put on the bottom of the library"
    );
    assert_eq!(
        counters_on(&engine, map, baylee_cards_dsl::counters::LANDMARK),
        1,
        "\"put a landmark counter on this artifact\""
    );
    assert_eq!(
        engine.state().object(map).map(|o| o.face_index),
        Some(0),
        "map remains on face 0 under Coverage::Partial"
    );
}

/// `Treasure Map` activated on three of its controller's turns carries three landmark
/// counters, and — with the three-counter clause unwritten — that is all: no Treasures and no
/// transform. That is the reason the clause stays off whole: its Treasures, written without
/// the counter removal and the transform in front of them, would be made by every activation
/// from the third on.
#[test]
fn treasure_map_gathers_three_landmark_counters_and_stops_there() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(268, forest())
        .battlefield(0, &[forest(), forest(), treasure_map()])
        .start();
    keep_mulligans(&mut engine);
    let map = on_battlefield(&engine, p0, treasure_map()).expect("map on battlefield");

    for activation in 1..=3 {
        assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
        tap_all_mana_but(&mut engine, p0, Some(treasure_map()));
        activate(&mut engine, p0, treasure_map(), 0);
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::Arrange { .. })
        });
        let Pending::Arrange { cards, .. } = engine.pending().clone() else {
            unreachable!("pass_until only stops on the scry")
        };
        engine.apply(p0, look_answer(&cards, &[])).unwrap();
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(
            counters_on(&engine, map, baylee_cards_dsl::counters::LANDMARK),
            activation,
            "one landmark counter per activation"
        );
        // Out of this turn, so the next walk finds the next one.
        pass_until(&mut engine, |e| e.state().turn.active != p0);
    }

    assert!(
        tokens_of(&engine, p0).is_empty(),
        "no Treasure is made while the clause in front of it is unwritten"
    );
    assert_eq!(
        engine.state().object(map).map(|o| o.face_index),
        Some(0),
        "and the map is still the map"
    );
}
