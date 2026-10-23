//! `cards/sorceries/mv_4/gerrard_s_wisdom.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn gerrard_s_wisdom() -> CardIndex {
    card_index("3e30e8f2-f437-4620-a3bc-dd9c29ae6570")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Gerrard's Wisdom (3e30e8f2-f437-4620-a3bc-dd9c29ae6570) — "You gain 2 life
/// for each card in your hand." The spell is on the stack when it resolves,
/// so it does not count itself: two Plains left in hand are +4, not +6. The
/// opponent's three-card hand counts for nothing and gains them nothing.
#[test]
fn gerrard_s_wisdom_gains_two_life_per_card_in_your_hand_not_theirs() {
    let mut e = lk::game(
        SEED,
        &[plains(), plains(), plains(), plains()],
        &[],
        &[gerrard_s_wisdom(), plains(), plains()],
        &[grizzly_bears(), grizzly_bears(), grizzly_bears()],
    );

    cast_from_hand(&mut e, P0, gerrard_s_wisdom());
    pass_until(&mut e, stack_is_empty);

    assert_eq!(lk::life(&e, P0), 24, "two cards in hand, 2 life each");
    assert_eq!(lk::life(&e, P1), 20, "the opponent gains nothing");
}

/// Gerrard's Wisdom — with an empty hand left behind the spell gains nothing:
/// the count is read when the spell resolves, not when it is cast.
#[test]
fn gerrard_s_wisdom_gains_nothing_with_an_empty_hand() {
    let mut e = lk::game(
        SEED,
        &[plains(), plains(), plains(), plains()],
        &[],
        &[gerrard_s_wisdom()],
        &[grizzly_bears()],
    );

    cast_from_hand(&mut e, P0, gerrard_s_wisdom());
    pass_until(&mut e, stack_is_empty);

    assert_eq!(lk::life(&e, P0), 20);
    assert_eq!(lk::life(&e, P1), 20);
}
