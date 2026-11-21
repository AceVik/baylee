//! `cards/instants/mv_2/worldly_counsel.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);

// oracle_id = "079b1f07-6910-491b-83ba-5ec41260f32d"
fn worldly_counsel() -> CardIndex {
    card_index("079b1f07-6910-491b-83ba-5ec41260f32d")
}

/// Worldly Counsel ({1}{U}, Domain): looks at top X cards where X is distinct
/// basic land types among lands YOU control (ignoring duplicates and
/// opponent's lands), puts one in hand, and puts the rest on bottom.
#[test]
fn worldly_counsel_looks_at_distinct_own_basic_land_types_and_bottoms_rest() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), forest(), plains()])
        .battlefield(1, &[swamp(), mountain()])
        .hand(0, &[worldly_counsel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let library = engine.state().zones.list(ZoneLocation::Library(P0)).clone();
    let fourth = library[library.len() - 4];
    let top_three = library[library.len() - 3..].to_vec();

    let islands = all_on_battlefield(&engine, P0, island());
    tap_mana_where(&mut engine, P0, |id| islands.contains(&id));
    cast_with_floating(&mut engine, P0, worldly_counsel());
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
        panic!("expected ChooseCards, got {:?}", engine.pending());
    };
    assert_eq!(player, P0);
    assert_eq!((min, max), (1, 1));
    assert_eq!(
        options.len(),
        3,
        "3 distinct basic land types (Island, Forest, Plains), ignoring duplicate Island and P1's lands"
    );
    assert!(options.iter().all(|id| top_three.contains(id)));
    assert!(!options.contains(&fourth));

    let chosen = options[0];
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .unwrap();
    let Pending::Arrange { cards, .. } = engine.pending().clone() else {
        panic!(
            "expected Arrange for bottom cards, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(cards.len(), 2, "two unchosen cards go to bottom");
    engine
        .apply(
            P0,
            PlayerAction::Arrange {
                piles: vec![cards.clone()],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(P0))
            .contains(&chosen),
        "chosen card moved to hand"
    );
    let after = engine.state().zones.list(ZoneLocation::Library(P0)).clone();
    assert_eq!(after.len(), library.len() - 1);
    assert!(
        after[..2].iter().all(|id| cards.contains(id)),
        "the other two cards are on the bottom"
    );
    assert_eq!(*after.last().unwrap(), fourth, "4th card is now on top");
}

/// With zero basic land types among lands you control, Domain is 0: resolves
/// cleanly without adding any card to hand.
#[test]
fn worldly_counsel_with_zero_basic_land_types_looks_at_nothing() {
    let mut engine = lk::game(SEED, &[], &[swamp(), mountain()], &[worldly_counsel()], &[]);
    let lib_before = engine.state().zones.list(ZoneLocation::Library(P0)).len();

    lk::float(&mut engine, P0, &[(ManaColor::Blue, 2)]);
    cast_with_floating(&mut engine, P0, worldly_counsel());
    lk::settle(&mut engine);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        0,
        "no card put into hand at Domain 0"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(P0)).len(),
        lib_before,
        "library unchanged at Domain 0"
    );
}
