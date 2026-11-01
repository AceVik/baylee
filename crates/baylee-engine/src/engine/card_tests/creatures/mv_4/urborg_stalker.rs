//! `cards/creatures/mv_4/urborg_stalker.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::Step;
use baylee_core::generated::index::{GRIZZLY_BEARS, LIGHTNING_BOLT, SCATHE_ZOMBIES};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn urborg_stalker() -> CardIndex {
    card_index("e5ef45e4-8c19-40bf-8756-c1b408dcb2fb")
}

/// Urborg Stalker — each upkeep checks that active player's permanents.
/// A green creature qualifies; a black creature and a colorless land do not.
#[test]
fn urborg_stalker_checks_only_the_active_players_nonblack_nonland_permanents() {
    let mut e = lk::game(
        SEED,
        &[urborg_stalker(), GRIZZLY_BEARS],
        &[SCATHE_ZOMBIES, forest()],
        &[],
        &[],
    );
    assert_eq!(lk::life(&e, P0), 19, "own green creature qualifies");
    assert_eq!(lk::life(&e, P1), 20);
    reach_their_main_phase(&mut e, P1);
    assert_eq!(
        lk::life(&e, P1),
        20,
        "black creature and nonblack land do not qualify"
    );
    assert_eq!(lk::life(&e, P0), 19, "P0 is not the active player");
    reach_their_main_phase(&mut e, P0);
    assert_eq!(lk::life(&e, P0), 18);
}

/// A qualifying opposing creature hurts its controller, not the Stalker's.
#[test]
fn urborg_stalker_damages_the_opponent_on_their_upkeep() {
    let mut e = lk::game(SEED, &[urborg_stalker()], &[GRIZZLY_BEARS], &[], &[]);
    assert_eq!(lk::life(&e, P0), 20, "only a black creature on this side");
    reach_their_main_phase(&mut e, P1);
    assert_eq!(lk::life(&e, P1), 19);
    assert_eq!(lk::life(&e, P0), 20);
}

/// The intervening 'if' is checked again when the trigger resolves: killing
/// the only nonblack nonland permanent in response prevents its damage.
#[test]
fn urborg_stalker_checks_the_condition_again_on_resolution() {
    let mut e = lk::game(
        SEED,
        &[urborg_stalker()],
        &[GRIZZLY_BEARS],
        &[LIGHTNING_BOLT],
        &[],
    );
    pass_until(&mut e, |e| {
        e.state().turn.active == P1
            && e.state().turn.step == Step::Upkeep
            && !stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    let bear = lk::obj(&e, P1, GRIZZLY_BEARS);
    lk::float(&mut e, P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut e, P0, LIGHTNING_BOLT);
    aim_at(&mut e, P0, bear);
    lk::settle(&mut e);
    assert!(in_graveyard(&e, P1, GRIZZLY_BEARS).is_some());
    assert_eq!(lk::life(&e, P1), 20, "no qualifying permanent remains");
}
