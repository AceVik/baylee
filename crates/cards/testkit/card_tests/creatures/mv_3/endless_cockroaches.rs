//! `cards/creatures/mv_3/endless_cockroaches.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn endless_cockroaches() -> CardIndex {
    card_index("9f31dbb1-c350-46f8-bd1d-9f23a073d2f1")
}

/// Endless Cockroaches ({1}{B}{B} 1/1 Insect):
/// "When this creature dies, return it to its owner's hand."
#[test]
fn endless_cockroaches_returns_to_owners_hand_when_it_dies_even_under_opponents_control() {
    let mut e = lk::game(
        SEED,
        &[index::SWAMP, index::SWAMP, index::SWAMP],
        &[],
        &[endless_cockroaches(), index::SHOCK, index::LIGHTNING_BOLT],
        &[],
    );
    cast_from_hand(&mut e, lk::P0, endless_cockroaches());
    lk::settle(&mut e);
    let bug = lk::obj(&e, lk::P0, endless_cockroaches());

    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut e, lk::P0, index::SHOCK, &[bug], &[]);
    assert!(in_hand(&e, lk::P0, endless_cockroaches()).is_some());
    assert!(in_graveyard(&e, lk::P0, endless_cockroaches()).is_none());

    // Recast it, give control to P1, and kill it: returns to owner P0's hand, not P1's.
    lk::float(&mut e, lk::P0, &[(ManaColor::Black, 3)]);
    cast_with_floating(&mut e, lk::P0, endless_cockroaches());
    lk::settle(&mut e);
    let bug2 = lk::obj(&e, lk::P0, endless_cockroaches());
    e.dev_state_mut(lk::P0)
        .unwrap()
        .object_mut(bug2)
        .unwrap()
        .set_controller(lk::P1);
    e.refresh_offer();

    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut e, lk::P0, index::LIGHTNING_BOLT, &[bug2], &[]);
    assert!(in_hand(&e, lk::P0, endless_cockroaches()).is_some());
    assert!(in_hand(&e, lk::P1, endless_cockroaches()).is_none());
}
