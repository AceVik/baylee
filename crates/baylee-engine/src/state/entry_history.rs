//! Entry facts are captured before later instructions can change the entrant.
use super::GameState;
use crate::object::GameObject;
use crate::zone::Zone;
use baylee_core::ids::{DamageSourceRef, ObjectId, PlayerId};
use std::sync::Arc;

/// A copy of `object` as it is now, its projected characteristics standing
/// in for its printed face, so nothing that happens later changes what it
/// was (CR 608.2h).
pub(crate) fn freeze_object(object: &GameObject) -> Arc<GameObject> {
    let mut frozen = object.clone();
    frozen.base = Arc::new(object.characteristics().clone());
    frozen.cache.clear();
    if frozen.status.contains(crate::object::Status::FACE_DOWN) {
        frozen.original_base = None;
        frozen.card = None;
        frozen.drop_own_abilities();
    }
    Arc::new(frozen)
}

/// The battlefield information a departure captured before any member of
/// its simultaneous event left (`BattlefieldDeparture`), laid over the
/// object's own last projection.
pub(crate) fn freeze_departure(
    object: &GameObject,
    departure: Option<&crate::damage_history::BattlefieldDeparture>,
) -> (Arc<GameObject>, PlayerId) {
    let Some(departure) = departure else {
        return (freeze_object(object), object.controller);
    };
    let mut frozen = object.clone();
    let mut chars = object.characteristics().clone();
    chars.subtypes = departure.subtypes;
    frozen.base = Arc::new(chars);
    frozen.cache.clear();
    frozen.controller = departure.event.controller;
    if frozen.status.contains(crate::object::Status::FACE_DOWN) {
        frozen.original_base = None;
        frozen.card = None;
        frozen.drop_own_abilities();
    }
    (Arc::new(frozen), departure.event.controller)
}

impl GameState {
    /// An arrival on the battlefield from `from` (`None`: created there).
    pub(crate) fn note_entry_history(&mut self, id: ObjectId, from: Option<Zone>) {
        if let Some(object) = self.object(id) {
            let reference = DamageSourceRef {
                object: id,
                version: object.version,
            };
            let (owner, controller) = (object.owner, object.controller);
            self.per_turn.pending_entry_history.push(reference);
            // The snapshot waits for every co-arrival, as the counts do.
            self.per_turn
                .zone_changes
                .items
                .push(super::ZoneChangeRecord {
                    object: reference,
                    from,
                    to: Zone::Battlefield,
                    owner,
                    controller,
                    snapshot: None,
                });
        }
    }

    /// A move of `id` out of `from` into a zone other than the battlefield,
    /// for [`super::PerTurn::zone_changes`]. `departed` is the object as it
    /// last existed in `from` when `from` was a public zone (`Battlefield`,
    /// `Graveyard`, `Stack`, `Command`, or face-up `Exile`).
    ///
    /// Moves strictly between hidden zones (`Hand <-> Library`, `OutsideGame`,
    /// or face-down exile, `CR 400.2`, `CR 406.3`) are never recorded because
    /// per-turn history is public (`state::fork`). Departures from a public
    /// zone into `Hand` or `Library` (*Barrin, Tolarian Archmage*'s
    /// battlefield-to-hand check, *Gau, Feral Youth*'s graveyard-departure
    /// check) ARE public events whose departing characteristics were visible
    /// to every seat at the table.
    pub(crate) fn note_zone_change(
        &mut self,
        id: ObjectId,
        from: Zone,
        departed: Option<(Arc<GameObject>, PlayerId)>,
    ) {
        let Some(object) = self.object(id) else {
            return;
        };
        let to = object.zone;
        let to_public = match to {
            Zone::Graveyard | Zone::Stack | Zone::Command => true,
            Zone::Exile => !object.status.contains(crate::object::Status::FACE_DOWN),
            Zone::Battlefield | Zone::Library | Zone::Hand | Zone::OutsideGame => false,
        };
        let from_public_to_hand_or_library = departed.is_some()
            && matches!(to, Zone::Hand | Zone::Library)
            && matches!(
                from,
                Zone::Battlefield | Zone::Graveyard | Zone::Stack | Zone::Command | Zone::Exile
            );
        if from == to
            || (!to_public && !from_public_to_hand_or_library)
            || object.kind == crate::object::ObjectKind::AbilityOnStack
            || (to_public && object.status.contains(crate::object::Status::FACE_DOWN))
        {
            return;
        }
        let (snapshot, controller) =
            departed.unwrap_or_else(|| (freeze_object(object), object.owner));
        let record = super::ZoneChangeRecord {
            object: DamageSourceRef {
                object: id,
                version: object.version,
            },
            from: (from != Zone::OutsideGame).then_some(from),
            to,
            owner: object.owner,
            controller,
            snapshot: Some(snapshot),
        };
        self.per_turn.zone_changes.items.push(record);
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
        let mut captured_snapshot = false;
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
            let frozen = freeze_object(object);
            if creature {
                let count = &mut self.per_turn.creatures_entered[usize::from(controller.get())];
                *count = count.saturating_add(1);
            }
            self.per_turn.entered_subtypes[usize::from(controller.get())].union_with(subtypes);
            if let Some(record) = self
                .per_turn
                .zone_changes
                .items
                .iter_mut()
                .rev()
                .find(|r| r.object == reference && r.snapshot.is_none())
            {
                record.controller = controller;
                record.snapshot = Some(frozen);
                captured_snapshot = true;
            }
        }
        self.per_turn.pending_entry_history = pending;
        if captured_snapshot && self.per_turn.zone_changes.tracked {
            self.sync_board_rules(lookup.as_ref());
            self.refresh_characteristics();
        }
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
