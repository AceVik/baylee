//! Three of the state's single doors, read through their contracts: the
//! tap door (its "did it change" answer and the stun counter, CR 122.1d),
//! the name table, and the checkpoint a reversed action rolls back to
//! (CR 732.1).

use super::{RegistryLookup, make_preset};
use crate::event::Cause;
use crate::object::{CounterKind, Status};
use crate::state::GameState;
use crate::zone::{ZoneLocation, ZonePosition};
use baylee_core::ids::{ObjectId, PlayerId};

const P0: PlayerId = PlayerId::new(0);

fn state(seed: u64) -> GameState {
    GameState::from_preset(&make_preset(seed), &RegistryLookup).expect("game starts")
}

fn permanent(state: &mut GameState) -> ObjectId {
    let card = state.draw_cards(P0, 1)[0];
    state
        .move_object(
            card,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap()
}

fn tapped(state: &GameState, id: ObjectId) -> bool {
    state.object(id).unwrap().status.contains(Status::TAPPED)
}

#[test]
fn tapping_says_whether_it_changed_anything() {
    let mut state = state(501);
    let id = permanent(&mut state);
    assert!(state.set_tapped(id, true), "untapped to tapped");
    assert!(!state.set_tapped(id, true), "already tapped");
    assert!(tapped(&state, id));
    assert!(state.set_tapped(id, false));
    assert!(!state.set_tapped(id, false), "already untapped");
    assert!(!tapped(&state, id));
}

/// CR 122.1d: a permanent with a stun counter that would become untapped
/// has one stun counter removed instead.
#[test]
fn a_stun_counter_is_removed_instead_of_untapping() {
    let mut state = state(502);
    let id = permanent(&mut state);
    state.set_tapped(id, true);
    state
        .object_mut(id)
        .unwrap()
        .counters
        .set(CounterKind::Stun, 2);
    assert!(!state.set_tapped(id, false));
    assert!(tapped(&state, id));
    assert_eq!(state.object(id).unwrap().counters.get(CounterKind::Stun), 1);
    assert!(!state.set_tapped(id, false));
    assert_eq!(state.object(id).unwrap().counters.get(CounterKind::Stun), 0);
    assert!(
        tapped(&state, id),
        "still tapped after the last stun counter"
    );
    assert!(state.set_tapped(id, false), "and the next untap untaps");
}

#[test]
fn a_stun_counter_does_not_stop_a_tap() {
    let mut state = state(503);
    let id = permanent(&mut state);
    state
        .object_mut(id)
        .unwrap()
        .counters
        .set(CounterKind::Stun, 1);
    assert!(state.set_tapped(id, true));
    assert_eq!(state.object(id).unwrap().counters.get(CounterKind::Stun), 1);
}

#[test]
fn a_stun_counter_on_an_untapped_permanent_is_not_spent_by_an_untap() {
    let mut state = state(504);
    let id = permanent(&mut state);
    state
        .object_mut(id)
        .unwrap()
        .counters
        .set(CounterKind::Stun, 1);
    assert!(!state.set_tapped(id, false));
    assert_eq!(state.object(id).unwrap().counters.get(CounterKind::Stun), 1);
}

#[test]
fn a_name_is_interned_once_and_found_after() {
    let mut state = state(505);
    assert_eq!(state.names.find("A Name Nobody Prints"), None);
    let before = state.names.len();
    let first = state.names.intern("A Name Nobody Prints");
    let again = state.names.intern("A Name Nobody Prints");
    assert_eq!(first, again);
    assert_eq!(state.names.len(), before + 1);
    assert_eq!(state.names.find("A Name Nobody Prints"), Some(first));
    assert_eq!(state.names.get(first), "A Name Nobody Prints");
}

#[test]
fn two_names_get_two_handles() {
    let mut state = state(506);
    let a = state.names.intern("Alpha Name");
    let b = state.names.intern("Beta Name");
    assert_ne!(a, b);
    assert_eq!(state.names.get(a), "Alpha Name");
    assert_eq!(state.names.get(b), "Beta Name");
}

#[test]
fn a_roll_back_puts_the_game_back_and_cuts_the_journal() {
    let mut state = state(507);
    let life = state.players[0].life;
    let journal = state.journal.len();
    let hash = state.snapshot_hash();
    let checkpoint = state.checkpoint();
    state.change_life(P0, -5, Cause::Effect);
    let _ = state.draw_cards(P0, 2);
    assert!(state.journal.len() > journal);
    state.roll_back(checkpoint);
    assert_eq!(state.players[0].life, life);
    assert_eq!(state.journal.len(), journal);
    assert_eq!(state.snapshot_hash(), hash);
}

#[test]
fn a_kept_checkpoint_can_be_rolled_back_to_twice() {
    let mut state = state(508);
    let hash = state.snapshot_hash();
    let checkpoint = state.checkpoint();
    for _ in 0..2 {
        state.change_life(P0, -3, Cause::Effect);
        state.roll_back_to(&checkpoint);
        assert_eq!(state.snapshot_hash(), hash);
    }
}
