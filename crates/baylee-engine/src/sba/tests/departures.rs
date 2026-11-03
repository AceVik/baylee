//! A player leaving a multiplayer game (CR 800.4a): everything they own
//! leaves with them, what they control but do not own is exiled, and the
//! others play on with their own things untouched.

use super::{RegistryLookup, bare, empty_table, run};
use crate::state::GameState;
use crate::zone::{Zone, ZoneLocation};
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_core::types::TypeSet;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
const P2: PlayerId = PlayerId::new(2);

/// A creature owned and controlled by `seat`.
fn owned_by(state: &mut GameState, seat: PlayerId) -> ObjectId {
    let id = bare(state, "Seat Creature", TypeSet::CREATURE, &[]);
    let obj = state.object_mut(id).unwrap();
    obj.owner = seat;
    obj.controller = seat;
    state.invalidate_projections();
    id
}

#[test]
fn a_losing_players_permanents_leave_the_game_with_them() {
    let mut state = empty_table(601, 3);
    let theirs = owned_by(&mut state, P1);
    let mine = owned_by(&mut state, P0);
    state.players[1].life = 0;
    let _ = run(&mut state, &RegistryLookup);
    assert!(state.players[1].has_lost());
    assert!(state.object(theirs).is_none(), "gone, not in a graveyard");
    assert!(state.object(mine).is_some());
    assert!(!state.players[0].has_lost());
    assert!(!state.players[2].has_lost());
}

#[test]
fn a_losing_players_library_and_hand_go_too() {
    let mut state = empty_table(602, 3);
    let _ = state.draw_cards(P1, 3);
    assert!(!state.zones.list(ZoneLocation::Library(P1)).is_empty());
    state.players[1].poison = 10;
    let _ = run(&mut state, &RegistryLookup);
    assert!(state.zones.list(ZoneLocation::Library(P1)).is_empty());
    assert!(state.zones.list(ZoneLocation::Hand(P1)).is_empty());
    assert!(
        !state.zones.list(ZoneLocation::Library(P2)).is_empty(),
        "another seat's library stays"
    );
}

#[test]
fn a_permanent_the_loser_controlled_without_an_effect_saying_so_is_its_owners() {
    // Control written straight onto the object, with no control-changing
    // effect behind it: the projection gives control back to the owner,
    // so when the seat it pointed at leaves, the permanent stays where it
    // is under the player who owns it (CR 800.4a ends only effects).
    let mut state = empty_table(603, 3);
    let borrowed = owned_by(&mut state, P0);
    state.object_mut(borrowed).unwrap().controller = P1;
    state.invalidate_projections();
    state.players[1].life = 0;
    let _ = run(&mut state, &RegistryLookup);
    let obj = state.object(borrowed).expect("the owner is still playing");
    assert_eq!(obj.zone, Zone::Battlefield);
    assert_eq!(obj.controller, P0);
}

#[test]
fn what_the_loser_owned_but_another_controlled_leaves_too() {
    let mut state = empty_table(604, 3);
    let lent = owned_by(&mut state, P1);
    state.object_mut(lent).unwrap().controller = P2;
    state.invalidate_projections();
    state.players[1].life = 0;
    let _ = run(&mut state, &RegistryLookup);
    assert!(state.object(lent).is_none());
}

#[test]
fn a_player_who_left_is_not_asked_to_lose_again() {
    let mut state = empty_table(605, 3);
    state.players[1].life = 0;
    let _ = run(&mut state, &RegistryLookup);
    state.players[1].poison = 10;
    assert!(!run(&mut state, &RegistryLookup).changed);
    assert_eq!(
        state.players[1].loss,
        Some(crate::event::LossReason::Life),
        "the first reason stays"
    );
}

#[test]
fn has_left_is_true_for_the_loser_only() {
    let mut state = empty_table(606, 4);
    state.players[2].life = 0;
    let _ = run(&mut state, &RegistryLookup);
    assert!(state.has_left(P2));
    for seat in [P0, P1, PlayerId::new(3)] {
        assert!(!state.has_left(seat), "{seat:?}");
    }
}
