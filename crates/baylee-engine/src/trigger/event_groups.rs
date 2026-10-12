//! Predicates over one declaration or one simultaneous damage event.
use crate::event::{GameEvent, JournalEntry};
use crate::state::GameState;
use crate::text_changes::RuleContext;
use baylee_cards_dsl::Filter;
use baylee_core::ids::PlayerId;

/// Recover an embedded event's row without searching the entire contiguous batch.
/// Address arithmetic never dereferences a pointer; the final identity check also
/// rejects external events and equal-valued copies. A subslice is its own batch.
pub(super) fn index_of_event(event: &GameEvent, batch: &[JournalEntry]) -> Option<usize> {
    let first = &batch.first()?.event;
    let offset = std::ptr::from_ref(event)
        .addr()
        .checked_sub(std::ptr::from_ref(first).addr())?;
    let stride = size_of::<JournalEntry>();
    if !offset.is_multiple_of(stride) {
        return None;
    }
    let index = offset / stride;
    batch
        .get(index)
        .filter(|entry| std::ptr::eq(&raw const entry.event, event))
        .map(|_| index)
}

pub(super) fn attack_threshold(
    event: &GameEvent,
    batch: &[JournalEntry],
    state: &GameState,
    you: PlayerId,
    minimum: u8,
) -> bool {
    let mut matching = batch.iter().filter(|entry| {
        let GameEvent::BecameAttacker { object, .. } = entry.event else {
            return false;
        };
        state.object(object).is_some_and(|object| {
            object.controller == you
                && object
                    .characteristics()
                    .types
                    .contains(baylee_core::types::TypeSet::CREATURE)
        })
    });
    matching.next().is_some_and(|first| {
        std::ptr::eq(&raw const first.event, event)
            && (minimum <= 1
                || matching.take(usize::from(minimum - 1)).count() == usize::from(minimum - 1))
    })
}

pub(super) fn first_matching_damage(
    event: &GameEvent,
    batch: &[JournalEntry],
    state: &GameState,
    you: PlayerId,
    context: RuleContext,
    filter: &Filter,
) -> bool {
    let GameEvent::DamageDealt { target, .. } = event else {
        return false;
    };
    batch
        .iter()
        .find(|entry| {
            let GameEvent::DamageDealt {
                source: Some(source),
                target: recipient,
                amount,
                is_combat: true,
            } = entry.event
            else {
                return false;
            };
            recipient == *target
                && amount > 0
                && state.object(source).is_some_and(|object| {
                    crate::eval::matches_with_context(filter, state, object, you, context)
                })
        })
        .is_some_and(|first| std::ptr::eq(&raw const first.event, event))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::Cause;
    use baylee_core::ids::ObjectId;

    #[test]
    fn indexed_event_lookup_matches_identity_scan_for_batches_and_subslices() {
        let entries: Vec<_> = (0..4096_u32)
            .map(|index| JournalEntry {
                seq: u64::from(index) + 1,
                // Many equal values ensure equality cannot substitute for identity.
                event: GameEvent::ObjectTapped {
                    object: ObjectId::new(index % 7, 0),
                    cause: Cause::Effect,
                },
                departure: None,
            })
            .collect();
        for slice in [
            &entries[..],
            &entries[317..],
            &entries[1024..2048],
            &entries[..0],
        ] {
            for entry in &entries {
                let reference = slice.iter().position(|candidate| {
                    std::ptr::eq(&raw const candidate.event, &raw const entry.event)
                });
                assert_eq!(index_of_event(&entry.event, slice), reference);
            }
            let external = entries[0].event.clone();
            assert_eq!(index_of_event(&external, slice), None);
        }
    }
}
