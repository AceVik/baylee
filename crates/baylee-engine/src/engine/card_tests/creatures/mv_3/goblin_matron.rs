//! `cards/creatures/mv_3/goblin_matron.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn goblin_matron() -> CardIndex {
    card_index("145737a7-c597-4dec-b752-207c2d0501e3")
}

fn cast_matron(e: &mut Engine<RegistryLookup>) -> (ObjectId, ObjectId) {
    let goblin = hand_to_library_top(e, lk::P0, index::RAGING_GOBLIN);
    let non_goblin = hand_to_library_top(e, lk::P0, index::GRIZZLY_BEARS);
    lk::float(e, lk::P0, &[(ManaColor::Red, 3)]);
    cast_with_floating(e, lk::P0, goblin_matron());
    pass_until(e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    (goblin, non_goblin)
}

#[test]
fn goblin_matron_reveals_a_goblin_into_hand_and_shuffles() {
    let mut e = lk::game(
        SEED,
        &[],
        &[],
        &[goblin_matron(), index::RAGING_GOBLIN, index::GRIZZLY_BEARS],
        &[],
    );
    let (goblin, non_goblin) = cast_matron(&mut e);
    lk::yes_no(&mut e, true);
    let Pending::ChooseCards {
        options, min, max, ..
    } = e.pending().clone()
    else {
        panic!("the optional search asks for a Goblin: {:?}", e.pending());
    };
    assert_eq!((min, max), (0, 1));
    assert!(options.contains(&goblin));
    assert!(!options.contains(&non_goblin));
    e.apply(
        lk::P0,
        PlayerAction::ChooseObjects {
            objects: vec![goblin],
        },
    )
    .unwrap();
    lk::settle(&mut e);
    assert!(in_hand(&e, lk::P0, index::RAGING_GOBLIN).is_some());
    assert!(
        !e.state()
            .zones
            .list(ZoneLocation::Library(lk::P0))
            .contains(&goblin)
    );
    assert!(e.state().journal.entries().iter().any(|entry| matches!(
        &entry.event,
        crate::event::GameEvent::Revealed { cards, .. } if cards.contains(&goblin)
    )));
    assert!(e.state().journal.entries().iter().any(|entry| matches!(
        entry.event,
        crate::event::GameEvent::Shuffled { player, .. } if player == lk::P0
    )));
}

#[test]
fn goblin_matron_may_decline_the_search() {
    let mut e = lk::game(
        SEED,
        &[],
        &[],
        &[goblin_matron(), index::RAGING_GOBLIN, index::GRIZZLY_BEARS],
        &[],
    );
    let (goblin, _) = cast_matron(&mut e);
    lk::yes_no(&mut e, false);
    lk::settle(&mut e);
    assert!(
        e.state()
            .zones
            .list(ZoneLocation::Library(lk::P0))
            .contains(&goblin)
    );
    assert!(in_hand(&e, lk::P0, index::RAGING_GOBLIN).is_none());
}
