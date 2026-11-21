//! `cards/instants/mv_3/machinate.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

// oracle_id = "545399a0-f1f5-40bc-bd40-038946df8ad7"
fn machinate() -> CardIndex {
    card_index("545399a0-f1f5-40bc-bd40-038946df8ad7")
}

/// Machinate ({1}{U}{U}): looks at the top X cards where X is the number of
/// artifacts YOU control (ignoring opponent's artifacts), puts 1 into hand,
/// and puts the rest on the bottom in any order.
#[test]
fn machinate_looks_at_top_x_by_your_artifacts_and_bottoms_rest() {
    let mut engine = lk::game(
        SEED,
        &[sol_ring(), index::ORNITHOPTER, quiet_artifact()],
        &[sol_ring(), quiet_artifact()],
        &[machinate()],
        &[],
    );
    let library = engine.state().zones.list(ZoneLocation::Library(P0)).clone();
    let fourth = library[library.len() - 4];
    let top_three = library[library.len() - 3..].to_vec();

    lk::float(&mut engine, P0, &[(ManaColor::Blue, 3)]);
    cast_with_floating(&mut engine, P0, machinate());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });

    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        unreachable!();
    };
    assert_eq!((min, max), (1, 1));
    assert_eq!(
        options.len(),
        3,
        "3 controlled artifacts (opponent's ignored)"
    );
    assert!(options.iter().all(|id| top_three.contains(id)));
    assert!(!options.contains(&fourth));

    let kept = options[0];
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![kept],
            },
        )
        .unwrap();
    let Pending::Arrange { cards, .. } = engine.pending().clone() else {
        panic!(
            "expected Arrange for bottom two, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(cards.len(), 2);
    engine
        .apply(
            P0,
            PlayerAction::Arrange {
                piles: vec![cards.clone()],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(P0))
            .contains(&kept)
    );
    let after = engine.state().zones.list(ZoneLocation::Library(P0)).clone();
    assert_eq!(*after.last().unwrap(), fourth);
}

/// With zero artifacts under your control, Machinate resolves cleanly without
/// adding any card to hand.
#[test]
fn machinate_with_zero_artifacts_looks_at_nothing() {
    let mut engine = lk::game(SEED, &[], &[sol_ring()], &[machinate()], &[]);
    let lib_before = engine.state().zones.list(ZoneLocation::Library(P0)).len();

    lk::float(&mut engine, P0, &[(ManaColor::Blue, 3)]);
    cast_with_floating(&mut engine, P0, machinate());
    lk::settle(&mut engine);

    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(P0)).len(), 0);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(P0)).len(),
        lib_before
    );
}
