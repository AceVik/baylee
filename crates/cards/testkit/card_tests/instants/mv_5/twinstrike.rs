//! `cards/instants/mv_5/twinstrike.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "6d890c2a-2ed7-429d-a2b2-d261397f838d"
fn twinstrike() -> CardIndex {
    card_index("6d890c2a-2ed7-429d-a2b2-d261397f838d")
}

/// Twinstrike ({3}{B}{R}): requires two distinct creature targets, deals 2
/// damage to each when you have cards in hand, and with Hellbent (0 cards in
/// hand on resolution) destroys both creatures instead.
#[test]
fn twinstrike_deals_two_each_with_hand_and_destroys_both_with_hellbent() {
    // Only 1 creature on the battlefield -> cannot cast (requires two targets).
    let mut one_target = lk::game(SEED, &[], &[index::SERRA_ANGEL], &[twinstrike()], &[]);
    lk::float(
        &mut one_target,
        P0,
        &[(ManaColor::Black, 4), (ManaColor::Red, 1)],
    );
    assert!(
        !lk::castable(&one_target, P0, twinstrike()),
        "cannot cast with fewer than two creatures on the battlefield"
    );

    // 1. Non-Hellbent (1 card left in hand): deals 2 damage to EACH of two 4/4s.
    let mut engine = lk::game(
        SEED,
        &[],
        &[index::SERRA_ANGEL, index::HILL_GIANT],
        &[twinstrike(), twinstrike()],
        &[],
    );
    let angel = lk::obj(&engine, P1, index::SERRA_ANGEL);
    let giant = lk::obj(&engine, P1, index::HILL_GIANT);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Black, 8), (ManaColor::Red, 2)],
    );
    cast_with_floating(&mut engine, P0, twinstrike());
    lk::aim(&mut engine, &[angel, giant], &[]);
    lk::settle(&mut engine);

    assert_eq!(engine.state().object(angel).unwrap().damage, 2);
    assert_eq!(engine.state().object(giant).unwrap().damage, 2);
    assert!(on_battlefield(&engine, P1, index::SERRA_ANGEL).is_some());
    assert!(on_battlefield(&engine, P1, index::HILL_GIANT).is_some());

    // 2. Hellbent (0 cards left in hand on 2nd cast): DESTROYS both (even 4/4 Angel).
    let mut hellbent = lk::game(
        SEED,
        &[],
        &[index::SERRA_ANGEL, index::SERRA_ANGEL],
        &[twinstrike()],
        &[],
    );
    let angels: Vec<_> = hellbent
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| hellbent.state().object(*id).unwrap().controller == P1)
        .collect();
    assert_eq!(angels.len(), 2);

    lk::float(
        &mut hellbent,
        P0,
        &[(ManaColor::Black, 4), (ManaColor::Red, 1)],
    );
    cast_with_floating(&mut hellbent, P0, twinstrike());
    lk::aim(&mut hellbent, &angels, &[]);
    lk::settle(&mut hellbent);

    assert!(
        on_battlefield(&hellbent, P1, index::SERRA_ANGEL).is_none(),
        "Hellbent destroyed both 4/4 Serra Angels"
    );
    assert_eq!(
        hellbent
            .state()
            .zones
            .list(ZoneLocation::Graveyard(P1))
            .len(),
        2
    );
}
