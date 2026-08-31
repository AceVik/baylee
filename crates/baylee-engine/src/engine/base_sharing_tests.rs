//! The printed face is shared, and writing to one splits it.
//!
//! A `GameObject` carries its printed characteristics behind an `Arc`, handed
//! out by `GameState`'s base cache. That is a performance decision — 528 bytes
//! per object became 272, and a board of thousands of tokens now reads one
//! face instead of thousands — but it is only *safe* because every write goes
//! through `GameObject::base_mut`, which splits the sharing first.
//!
//! Both halves fail silently. If a future creation path builds a face of its
//! own, nothing breaks; the engine just gets slow again in a way no test would
//! notice. If a write reaches a shared face without splitting it, one Forest
//! becoming a Zombie turns every Forest in the deck into one — and that is not
//! a slowdown, it is a wrong game. So both are pinned here.

use super::testkit::{Duel, card_index, keep_mulligans};
use super::*;
use crate::object::{Characteristics, ObjectKind};
use crate::zone::ZoneLocation;
use baylee_core::ids::{CardIndex, ObjectId};
use std::sync::Arc;

fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}

fn zombie() -> baylee_core::ids::SubtypeId {
    baylee_core::generated::subtypes::creature::ZOMBIE
}

/// Which allocation an object's printed face lives in. Identity, not equality:
/// two objects that merely *look* alike would pass an `==` and prove nothing.
fn face_of(state: &GameState, id: ObjectId) -> *const Characteristics {
    Arc::as_ptr(&state.object(id).expect("object exists").base)
}

/// Every copy of the same card in a library points at one printed face.
#[test]
fn every_copy_of_a_card_shares_one_printed_face() {
    let engine = Duel::new(3, forest()).start();
    let state = engine.state();
    let library = state.zones.list(ZoneLocation::Library(PlayerId::new(0)));
    assert!(library.len() > 10, "a library worth comparing across");

    let first = face_of(state, library[0]);
    for &id in library {
        assert_eq!(
            face_of(state, id),
            first,
            "a deck of one card name must not allocate a face per copy"
        );
    }
}

/// The blank face behind a card-less object is shared too. This is the one
/// that has to survive six figures of triggers on the stack.
#[test]
fn card_less_objects_share_the_blank_face_of_their_name() {
    let mut engine = Duel::new(3, forest()).start();
    let state = engine.state_mut_dev();
    let owner = PlayerId::new(0);
    let name = state.names.intern("Ally trigger");
    let ids: Vec<ObjectId> = (0..64)
        .map(|_| state.create_bare(owner, ObjectKind::AbilityOnStack, name, ZoneLocation::Stack))
        .collect();

    let first = face_of(state, ids[0]);
    for &id in &ids {
        assert_eq!(
            face_of(state, id),
            first,
            "abilities of one name must not allocate a face each"
        );
    }

    // A different name is a different face: the name is the only thing a
    // blank face carries, so sharing across names would lose it.
    let other = state.names.intern("Other trigger");
    let other_id = state.create_bare(
        owner,
        ObjectKind::AbilityOnStack,
        other,
        ZoneLocation::Stack,
    );
    assert_ne!(face_of(state, other_id), first);
    assert_eq!(
        state
            .object(other_id)
            .expect("ability exists")
            .characteristics()
            .name,
        other
    );
}

/// Writing splits: the writer gets its own face and every sharer keeps the
/// old one.
#[test]
fn writing_a_base_leaves_the_other_sharers_alone() {
    let mut engine = Duel::new(3, forest()).start();
    let state = engine.state_mut_dev();
    let library = state
        .zones
        .list(ZoneLocation::Library(PlayerId::new(0)))
        .clone();
    let (victim, bystander) = (library[0], library[1]);
    assert_eq!(face_of(state, victim), face_of(state, bystander));

    state
        .object_mut(victim)
        .expect("card exists")
        .base_mut()
        .subtypes
        .insert(zombie());

    let subtypes = |id| {
        state
            .object(id)
            .expect("card exists")
            .characteristics()
            .subtypes
    };
    assert!(subtypes(victim).contains(zombie()), "the write landed");
    assert!(
        !subtypes(bystander).contains(zombie()),
        "and reached nothing else"
    );
    assert_ne!(
        face_of(state, victim),
        face_of(state, bystander),
        "the writer took a face of its own"
    );
}

/// Cloning a state — the AI's per-ply primitive — copies handles, not faces,
/// and a write in the copy still cannot reach the original.
#[test]
fn a_cloned_state_keeps_sharing_the_faces_of_the_original() {
    let mut engine = Duel::new(3, forest()).start();
    keep_mulligans(&mut engine);
    let original = engine.state().clone();
    let mut copy = original.clone();

    let library = original
        .zones
        .list(ZoneLocation::Library(PlayerId::new(0)))
        .clone();
    for &id in library.iter().take(8) {
        assert_eq!(
            face_of(&original, id),
            face_of(&copy, id),
            "the clone must not deep-copy a face the original still holds"
        );
    }

    let victim = library[0];
    copy.object_mut(victim)
        .expect("card exists")
        .base_mut()
        .subtypes
        .insert(zombie());
    assert!(
        !original
            .object(victim)
            .expect("card exists")
            .characteristics()
            .subtypes
            .contains(zombie()),
        "a write in the copy must not reach the state it was cloned from"
    );
}

/// The handle is what makes an object cheap to copy; `tests/footprint.rs`
/// asserts the number this buys.
#[test]
fn the_printed_face_costs_a_pointer_in_the_object() {
    assert_eq!(
        size_of::<Arc<Characteristics>>(),
        size_of::<usize>(),
        "a handle, not an inline copy"
    );
}
