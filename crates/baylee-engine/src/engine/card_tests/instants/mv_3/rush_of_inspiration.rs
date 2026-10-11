//! `cards/instants/mv_3/rush_of_inspiration.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Without energy the spell draws two, then discards one at random.
#[test]
fn rush_of_inspiration_draws_two_cards() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(44, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[rush_of_inspiration()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    cast_from_hand(&mut engine, p0, rush_of_inspiration());
    pass_until(&mut engine, stack_is_empty);

    let hand_after = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    assert_eq!(
        hand_after, hand_before,
        "the spell and random discard cost two cards, and two are drawn"
    );
    assert!(
        in_graveyard(&engine, p0, rush_of_inspiration()).is_some(),
        "resolved spell is in the graveyard"
    );
}
