//! `cards/creatures/mv_3/necratog.rs`, played.
//!
//! The ability is left off the card (the exile cost cannot name "the top
//! creature card of your graveyard"), so this pins the body and that no
//! pump is on offer. When the cost exists, this test gives way to the
//! pump tests: +2/+2 until end of turn, the top card only, nothing to exile.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);

fn necratog() -> CardIndex {
    card_index("9f820f1e-3cd1-4a3e-8d00-194c8a80b296")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Necratog (9f820f1e-3cd1-4a3e-8d00-194c8a80b296) — a 1/2 for {1}{B}{B}; with
/// a creature card in the graveyard no activated ability is offered, because
/// the card carries none rather than one that exiles a card it should not.
#[test]
fn necratog_is_a_one_two_body_without_the_unmodelled_pump() {
    let mut e = lk::game(
        SEED,
        &[swamp(), swamp(), swamp()],
        &[],
        &[necratog(), grizzly_bears()],
        &[],
    );
    hand_to_graveyard(&mut e, P0, grizzly_bears());

    cast_from_hand(&mut e, P0, necratog());
    pass_until(&mut e, stack_is_empty);

    let body = lk::obj(&e, P0, necratog());
    assert_eq!(pt(&e, body), (1, 2));
    assert!(!lk::offered(&e, necratog(), P0, 0), "no ability to offer");
    assert!(
        in_graveyard(&e, P0, grizzly_bears()).is_some(),
        "nothing was exiled"
    );
}
