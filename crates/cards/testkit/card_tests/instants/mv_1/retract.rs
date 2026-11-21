//! `cards/instants/mv_1/retract.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "31cefc9d-cc13-4b32-a1f8-edbcaf35aa3b"
fn retract() -> CardIndex {
    card_index("31cefc9d-cc13-4b32-a1f8-edbcaf35aa3b")
}

/// Retract ({U}): returns ALL artifacts you control (both noncreature and
/// artifact creatures) to their owners' hands, leaving your non-artifacts and
/// opponent's artifacts on the battlefield.
#[test]
fn retract_returns_only_your_artifacts_to_their_owners_hands() {
    let mut engine = lk::game(
        SEED,
        &[sol_ring(), index::ORNITHOPTER, index::GRIZZLY_BEARS],
        &[sol_ring()],
        &[retract()],
        &[],
    );
    let my_bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let their_ring = lk::obj(&engine, P1, sol_ring());

    lk::float(&mut engine, P0, &[(ManaColor::Blue, 1)]);
    cast_with_floating(&mut engine, P0, retract());
    lk::settle(&mut engine);

    assert!(in_hand(&engine, P0, sol_ring()).is_some());
    assert!(in_hand(&engine, P0, index::ORNITHOPTER).is_some());
    assert_eq!(
        on_battlefield(&engine, P0, index::GRIZZLY_BEARS),
        Some(my_bears),
        "non-artifact creature stays on battlefield"
    );
    assert_eq!(
        on_battlefield(&engine, P1, sol_ring()),
        Some(their_ring),
        "opponent's artifact stays on battlefield"
    );
}
