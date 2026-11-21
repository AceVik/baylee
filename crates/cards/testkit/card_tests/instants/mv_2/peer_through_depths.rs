//! `cards/instants/mv_2/peer_through_depths.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

// oracle_id = "a61f5100-aa38-4996-b23c-3ce4ed25ed09"
fn peer_through_depths() -> CardIndex {
    card_index("a61f5100-aa38-4996-b23c-3ce4ed25ed09")
}

/// Peer Through Depths ({1}{U}): looks at the top 5 cards of your library,
/// offers ONLY instant/sorcery cards among them (allowing 0 or 1), puts the
/// chosen card into your hand, and puts the rest on the bottom in any order.
#[test]
fn peer_through_depths_picks_only_instant_or_sorcery_and_bottoms_rest() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[],
        &[
            peer_through_depths(),
            lightning_bolt(),
            index::GRIZZLY_BEARS,
        ],
        &[],
    );
    let bolt = in_hand(&engine, P0, lightning_bolt()).unwrap();
    let bears = in_hand(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    for id in [bears, bolt] {
        engine
            .dev_state_mut(P0)
            .unwrap()
            .move_object(
                id,
                ZoneLocation::Library(P0),
                baylee_engine::zone::ZonePosition::Top,
                baylee_engine::event::Cause::Effect,
            )
            .unwrap();
    }
    let library = engine.state().zones.list(ZoneLocation::Library(P0)).clone();
    let sixth = library[library.len() - 6];

    lk::float(&mut engine, P0, &[(ManaColor::Blue, 2)]);
    cast_with_floating(&mut engine, P0, peer_through_depths());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });

    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseCards, got {:?}", engine.pending());
    };
    assert_eq!((min, max), (0, 1), "may reveal one instant or sorcery");
    assert_eq!(
        options,
        vec![bolt],
        "only the instant among top 5 is offered (Bears and Forests excluded)"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![bolt],
            },
        )
        .unwrap();

    let Pending::Arrange { cards, .. } = engine.pending().clone() else {
        panic!(
            "expected Arrange for remaining 4, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(cards.len(), 4);
    engine
        .apply(
            P0,
            PlayerAction::Arrange {
                piles: vec![cards.clone()],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    assert!(in_hand(&engine, P0, lightning_bolt()).is_some());
    let after = engine.state().zones.list(ZoneLocation::Library(P0)).clone();
    assert_eq!(after.len(), library.len() - 1);
    assert_eq!(*after.last().unwrap(), sixth, "6th card is now on top");
}
