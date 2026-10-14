//! State identity and arena sharing across deathtouch-flag reset.

use super::{RegistryLookup, creature, empty_boards_preset, on_battlefield, run};
use crate::object::Status;
use crate::state::GameState;
use baylee_cards_dsl::KeywordSet;
use baylee_core::ids::ObjectId;

fn quiet_board() -> (GameState, Vec<ObjectId>) {
    let mut state = GameState::from_preset(&empty_boards_preset(43), &RegistryLookup).unwrap();
    let creatures: Vec<_> = (0..8).map(|_| creature(&mut state)).collect();
    state.refresh_characteristics();
    (state, creatures)
}

fn fingerprint(state: &GameState) -> Vec<(&'static str, String)> {
    let mut fields = Vec::new();
    state.fingerprint(true, &mut fields);
    fields
}

fn assert_no_sba_action(state: &mut GameState) {
    let outcome = run(state, &RegistryLookup);
    assert!(!outcome.changed);
    assert!(outcome.legend_choice.is_none());
    assert!(outcome.commander_zone.is_none());
}

fn assert_same_state(actual: &GameState, expected: &GameState) {
    assert_eq!(fingerprint(actual), fingerprint(expected));
    let hash = actual.snapshot_hash();
    assert_eq!(hash, expected.snapshot_hash());
    // A clone has no snapshot memo: compare a warmed memo with a
    // fresh serialization, then exercise the warmed memo again.
    assert_eq!(hash, actual.clone().snapshot_hash());
    assert_eq!(hash, actual.snapshot_hash());
}

#[test]
fn an_unmarked_board_keeps_every_chunk_shared_with_the_checkpoint() {
    let (mut state, creatures) = quiet_board();
    state
        .object_mut(creatures[0])
        .unwrap()
        .status
        .insert(Status::PHASED_OUT);
    assert_no_sba_action(&mut state);
    let before_hash = state.snapshot_hash();
    let checkpoint = state.clone();
    let keys: Vec<_> = checkpoint
        .arena
        .chunk_views()
        .map(|view| view.key())
        .collect();
    assert!(!keys.is_empty(), "the sharing check must visit real chunks");
    assert!(
        checkpoint
            .arena
            .iter()
            .all(|(_, object)| !object.deathtouched),
        "there must be no flag that legitimately requires a write"
    );

    // Two settled passes cover both the first pass with a held
    // checkpoint and repeated checks of the same unchanged board.
    for _ in 0..2 {
        assert_no_sba_action(&mut state);
        assert_same_state(&state, &checkpoint);
        assert_eq!(state.snapshot_hash(), before_hash);
        let views: Vec<_> = state.arena.chunk_views().collect();
        assert_eq!(views.len(), keys.len(), "no chunk disappeared");
        for (index, (view, key)) in views.iter().zip(&keys).enumerate() {
            assert!(view.is(key), "untouched arena chunk {index} was copied");
        }
    }
}

#[test]
fn marked_survivors_reset_without_changing_other_state_or_the_checkpoint() {
    let (mut state, creatures) = quiet_board();
    let indestructible = creatures[1];
    let phased_out = creatures[6];
    state
        .object_mut(indestructible)
        .unwrap()
        .base_mut()
        .keywords = KeywordSet::INDESTRUCTIBLE;
    state.refresh_characteristics();
    state
        .object_mut(phased_out)
        .unwrap()
        .status
        .insert(Status::PHASED_OUT);
    for id in [indestructible, phased_out] {
        let object = state.object_mut(id).unwrap();
        object.damage = 1;
        object.deathtouched = true;
    }
    let before_hash = state.snapshot_hash();
    let before_fields = fingerprint(&state);
    let checkpoint = state.clone();
    let mut expected = checkpoint.clone();
    // The existing indestructible/phasing behavior keeps both
    // objects alive. This pass should change only their flags.
    for id in [indestructible, phased_out] {
        expected.object_mut(id).unwrap().deathtouched = false;
    }

    assert_no_sba_action(&mut state);
    assert_same_state(&state, &expected);
    assert_ne!(state.snapshot_hash(), before_hash, "the reset is hashed");
    assert_eq!(checkpoint.snapshot_hash(), before_hash);
    assert_eq!(fingerprint(&checkpoint), before_fields);
    for id in creatures {
        assert!(on_battlefield(&state, id));
        let object = state.object(id).unwrap();
        assert!(!object.deathtouched);
        if [indestructible, phased_out].contains(&id) {
            assert_eq!(object.damage, 1, "resetting the flag preserves damage");
            assert!(checkpoint.object(id).unwrap().deathtouched);
        }
    }
    // A second pass must not revive the marks or alter any
    // fingerprint field, including the event journal.
    assert_no_sba_action(&mut state);
    assert_same_state(&state, &expected);
}
