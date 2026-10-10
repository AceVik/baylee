//! `cards/artifacts/mv_2/treasure_map.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

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
        "the first landmark does not transform"
    );
}

#[test]
fn treasure_map_transforms_once_removes_counters_and_creates_three_treasures() {
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
            if activation == 3 { 0 } else { activation },
            "one landmark counter per activation"
        );
        pass_until(&mut engine, |e| e.state().turn.active != p0);
    }

    assert!(
        tokens_of(&engine, p0).len() == 3,
        "three Treasures are made on the third activation"
    );
    assert_eq!(
        engine.state().object(map).map(|o| o.face_index),
        Some(1),
        "the map is now Treasure Cove"
    );
}

#[test]
fn treasure_map_removes_all_landmarks_and_doubles_only_the_created_treasures() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                treasure_map(),
                forest(),
                baylee_core::generated::index::DOUBLING_SEASON,
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let map = on_battlefield(&e, p0, treasure_map()).unwrap();
    let version = e.state().object(map).unwrap().version;
    let state = e.dev_state_mut(p0).unwrap();
    crate::replacement::put_counters(state, map, baylee_cards_dsl::counters::LANDMARK, 1);
    crate::replacement::put_counters(state, map, CounterKind::P1P1, 1);
    tap_all_mana_but(&mut e, p0, Some(treasure_map()));
    activate(&mut e, p0, treasure_map(), 0);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::Arrange { .. }));
    let Pending::Arrange { cards, .. } = e.pending().clone() else {
        unreachable!()
    };
    e.apply(p0, look_answer(&cards, &[])).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        counters_on(&e, map, baylee_cards_dsl::counters::LANDMARK),
        0
    );
    assert_eq!(counters_on(&e, map, CounterKind::P1P1), 2);
    assert_eq!(tokens_of(&e, p0).len(), 6);
    assert_eq!(e.state().object(map).unwrap().version, version);
    assert_eq!(e.state().object(map).unwrap().face_index, 1);
    assert!(is_tapped(&e, map));
}
