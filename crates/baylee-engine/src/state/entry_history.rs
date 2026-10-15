//! Entry facts are captured before later instructions can change the entrant.
use super::GameState;
use crate::zone::Zone;
use baylee_core::ids::{DamageSourceRef, ObjectId};

impl GameState {
    pub(crate) fn note_entry_history(&mut self, id: ObjectId) {
        if let Some(object) = self.object(id) {
            let reference = DamageSourceRef {
                object: id,
                version: object.version,
            };
            self.per_turn.pending_entry_history.push(reference);
        }
    }

    /// Wait until co-arrivals are present, then freeze projected subtypes and controller.
    pub(crate) fn capture_entry_history(&mut self) {
        if self.per_turn.pending_entry_history.is_empty() {
            return;
        }
        let lookup = self.bases.clone();
        self.sync_board_rules(lookup.as_ref());
        self.refresh_characteristics();
        let mut pending = std::mem::take(&mut self.per_turn.pending_entry_history);
        for reference in pending.drain(..) {
            let Some(object) = self
                .object(reference.object)
                .filter(|o| o.zone == Zone::Battlefield && o.version == reference.version)
            else {
                continue;
            };
            let controller = object.controller;
            let creature = object
                .characteristics()
                .types
                .contains(baylee_core::types::TypeSet::CREATURE);
            let subtypes = object.characteristics().subtypes;
            if creature {
                let count = &mut self.per_turn.creatures_entered[usize::from(controller.get())];
                *count = count.saturating_add(1);
            }
            self.per_turn.entered_subtypes[usize::from(controller.get())].union_with(subtypes);
        }
        self.per_turn.pending_entry_history = pending;
    }
}

#[cfg(test)]
mod tests {
    use crate::engine::synthetic::{SyntheticLookup, preset};
    use crate::{
        event::Cause,
        object::ObjectKind,
        state::GameState,
        zone::{ZoneLocation, ZonePosition},
    };
    use baylee_core::{ids::PlayerId, types::TypeSet};

    #[test]
    fn creature_arrivals_keep_entry_controller_and_count_returned_incarnations() {
        let mut state =
            GameState::from_preset(&preset(573, &[]), &SyntheticLookup::new(vec![])).unwrap();
        let owner = PlayerId::new(0);
        let controller = PlayerId::new(1);
        let name = state.names.intern("historical entrant");
        let creature = state.create_bare(
            owner,
            ObjectKind::Permanent,
            name,
            ZoneLocation::Exile(owner),
        );
        let object = state.object_mut(creature).unwrap();
        std::sync::Arc::make_mut(&mut object.base).types = TypeSet::CREATURE;
        object.set_controller(controller);
        let _ = state.move_object(
            creature,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        );
        state.capture_entry_history();
        assert_eq!(state.per_turn.creatures_entered, [0, 1]);
        let _ = state.move_object(
            creature,
            ZoneLocation::Exile(owner),
            ZonePosition::Top,
            Cause::Effect,
        );
        assert_eq!(state.per_turn.creatures_entered, [0, 1]);
        state.object_mut(creature).unwrap().set_controller(owner);
        let _ = state.move_object(
            creature,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        );
        state.capture_entry_history();
        assert_eq!(state.per_turn.creatures_entered, [1, 1]);
        state.capture_entry_history();
        assert_eq!(
            state.per_turn.creatures_entered,
            [1, 1],
            "no double counting when projection repeats"
        );
        let _ = state.move_object(
            creature,
            ZoneLocation::Graveyard(owner),
            ZonePosition::Top,
            Cause::Effect,
        );
        assert_eq!(state.per_turn.creatures_entered, [1, 1]);
        state.per_turn.reset();
        assert_eq!(state.per_turn.creatures_entered, [0, 0]);
    }
}
