//! `cards/creatures/mv_3/pyre_zombie.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn pyre_zombie() -> CardIndex {
    card_index("accb82be-4f90-4d9a-bafd-f8de3622b3a3")
}

/// Pyre Zombie ({1}{B}{R} 2/1 Zombie):
/// "At the beginning of your upkeep, if this card is in your graveyard, you may
/// pay {1}{B}{B}. If you do, return it to your hand.
/// {1}{R}{R}, Sacrifice this creature: It deals 2 damage to any target."
#[test]
fn pyre_zombie_sacrifices_for_two_damage_and_may_pay_at_your_upkeep_to_return_to_hand() {
    let mut e = lk::game(
        SEED,
        &[pyre_zombie(), index::SWAMP, index::SWAMP, index::SWAMP],
        &[index::GRIZZLY_BEARS],
        &[],
        &[],
    );
    let bear = lk::obj(&e, lk::P1, index::GRIZZLY_BEARS);

    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 3)]);
    lk::use_ability(&mut e, lk::P0, pyre_zombie(), 0, &[bear], &[]);
    assert!(in_graveyard(&e, lk::P1, index::GRIZZLY_BEARS).is_some());
    assert!(in_graveyard(&e, lk::P0, pyre_zombie()).is_some());

    // Opponent's upkeep does not trigger Pyre Zombie.
    reach_their_main_phase(&mut e, lk::P1);
    assert!(in_graveyard(&e, lk::P0, pyre_zombie()).is_some());

    // P0's next upkeep: decline {1}{B}{B} -> stays in graveyard.
    pass_until(&mut e, |eng| matches!(eng.pending(), Pending::YesNo { .. }));
    lk::yes_no(&mut e, false);
    reach_main_phase(&mut e, lk::P0);
    assert!(in_graveyard(&e, lk::P0, pyre_zombie()).is_some());

    // Following P0 upkeep: float {1}{B}{B} when trigger is on stack, then accept YesNo(true).
    reach_their_main_phase(&mut e, lk::P1);
    pass_until(&mut e, |eng| {
        eng.state().turn.active == lk::P0 && matches!(eng.pending(), Pending::YesNo { .. })
    });
    let Pending::YesNo { player, .. } = e.pending().clone() else {
        unreachable!()
    };
    assert_eq!(player, lk::P0);
    // Tap the 3 Swamps via auto-tap / or pay after YesNo(true): check both auto-tap or payment window!
    e.apply(lk::P0, PlayerAction::YesNo(true)).unwrap();
    if matches!(e.pending(), Pending::Priority { .. }) && !stack_is_empty(&e) {
        tap_all_mana(&mut e, lk::P0);
    }
    reach_main_phase(&mut e, lk::P0);
    assert!(in_hand(&e, lk::P0, pyre_zombie()).is_some());
    assert!(in_graveyard(&e, lk::P0, pyre_zombie()).is_none());
}
