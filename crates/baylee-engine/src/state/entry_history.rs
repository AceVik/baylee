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
            let subtypes = object.characteristics().subtypes;
            self.per_turn.entered_subtypes[usize::from(controller.get())].union_with(subtypes);
        }
        self.per_turn.pending_entry_history = pending;
    }
}
