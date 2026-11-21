//! `cards/instants/mv_5/prophetic_bolt.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

// oracle_id = "d5e4bf5e-1a66-4315-8c82-640c27977b88"
fn prophetic_bolt() -> CardIndex {
    card_index("d5e4bf5e-1a66-4315-8c82-640c27977b88")
}

/// Prophetic Bolt ({3}{U}{R}): deals 4 damage to any target, then looks at the
/// top four cards of your library, puts one into your hand, and puts the rest
/// on the bottom of your library in any order.
#[test]
fn prophetic_bolt_deals_four_and_impulses_top_four() {
    let mut engine = table(
        SEED,
        &[prophetic_bolt()],
        &[island(), island(), island(), island(), mountain()],
        &[],
        &[],
    );
    let library = engine
        .state()
        .zones
        .list(ZoneLocation::Library(me()))
        .clone();
    let fifth = library[library.len() - 5];
    let top_four = library[library.len() - 4..].to_vec();

    tap_all_mana(&mut engine, me());
    cast_with_floating(&mut engine, me(), prophetic_bolt());
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![them()],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });

    assert_eq!(life_of(&engine, them()), 16, "4 damage dealt to opponent");
    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        unreachable!();
    };
    assert_eq!((min, max), (1, 1));
    assert_eq!(options.len(), 4);
    assert!(options.iter().all(|id| top_four.contains(id)));
    assert!(!options.contains(&fifth));

    let kept = options[0];
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![kept],
            },
        )
        .unwrap();
    let Pending::Arrange { cards, .. } = engine.pending().clone() else {
        panic!(
            "expected Arrange for bottom three, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(cards.len(), 3);
    engine
        .apply(
            me(),
            PlayerAction::Arrange {
                piles: vec![cards.clone()],
            },
        )
        .unwrap();
    settle(&mut engine);

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(me()))
            .contains(&kept)
    );
    let after = engine
        .state()
        .zones
        .list(ZoneLocation::Library(me()))
        .clone();
    assert_eq!(after.len(), library.len() - 1);
    assert!(after[..3].iter().all(|id| cards.contains(id)));
    assert_eq!(*after.last().unwrap(), fifth, "5th card is now on top");
}
