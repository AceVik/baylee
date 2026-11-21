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
    filter: Option<(&Filter, RuleContext)>,
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
                && filter.is_none_or(|(filter, context)| {
                    crate::eval::matches_with_context(filter, state, object, you, context)
                })
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
                    crate::eval::matches_damage_source_with_context(
                        filter, state, object, you, context,
                    )
                })
        })
        .is_some_and(|first| std::ptr::eq(&raw const first.event, event))
}

pub(super) const fn zone_ref_matches(
    zone_ref: baylee_cards_dsl::ZoneRef,
    zone: crate::zone::Zone,
) -> bool {
    use crate::zone::Zone as Z;
    use baylee_cards_dsl::ZoneRef as R;
    match zone_ref {
        R::Battlefield => matches!(zone, Z::Battlefield),
        R::Stack => matches!(zone, Z::Stack),
        R::Library => matches!(zone, Z::Library),
        R::Hand => matches!(zone, Z::Hand),
        R::Graveyard => matches!(zone, Z::Graveyard),
        R::Exile => matches!(zone, Z::Exile),
        R::Command => matches!(zone, Z::Command),
        R::NotBattlefield => !matches!(zone, Z::Battlefield),
        R::BattlefieldOrCommand => matches!(zone, Z::Battlefield | Z::Command),
    }
}

/// CR 603.10a: zone-batch triggers that look back to the game state
/// immediately before the event (leaves-the-battlefield, leaves-a-graveyard,
/// or public-zone object put into a hand or library).
pub(super) const fn changes_zone_all_looks_back(
    origin: Option<baylee_cards_dsl::ZoneRef>,
    destination: Option<baylee_cards_dsl::ZoneRef>,
) -> bool {
    use baylee_cards_dsl::ZoneRef as R;
    (matches!(origin, Some(R::Battlefield)) && !matches!(destination, Some(R::Battlefield)))
        || matches!(origin, Some(R::Graveyard))
        || (matches!(destination, Some(R::Hand | R::Library))
            && matches!(
                origin,
                Some(R::Battlefield | R::Graveyard | R::Exile | R::Stack | R::Command)
            ))
}

const fn breaks_zone_run(event: &GameEvent) -> bool {
    matches!(
        event,
        GameEvent::CardsDrawn { .. }
            | GameEvent::SpellCast { .. }
            | GameEvent::SpellCopied { .. }
            | GameEvent::AbilityActivated { .. }
            | GameEvent::AbilityTriggered { .. }
            | GameEvent::StepChanged { .. }
            | GameEvent::TurnStarted { .. }
            | GameEvent::DamageDealt { .. }
            | GameEvent::LifeChanged { .. }
            | GameEvent::LibrarySearched { .. }
            | GameEvent::Shuffled { .. }
            | GameEvent::CoinFlipped { .. }
            | GameEvent::DiceRolled { .. }
            | GameEvent::ManaProduced { .. }
    )
}

/// Count matching zone changes in the contiguous action run containing `event`
/// (`CR 603.2c`, `CR 603.10a`), returning that batch count only on the first
/// matching `ZoneChanged` entry of the run (and `0` on subsequent entries).
#[allow(clippy::too_many_arguments)]
pub(super) fn zone_batch_count(
    event: &GameEvent,
    batch: &[JournalEntry],
    state: &GameState,
    you: PlayerId,
    context: RuleContext,
    origin: Option<baylee_cards_dsl::ZoneRef>,
    destination: Option<baylee_cards_dsl::ZoneRef>,
    filter: &Filter,
    exclude_self: bool,
) -> u32 {
    if !matches!(event, GameEvent::ZoneChanged { .. }) {
        return 0;
    }
    let Some(idx) = index_of_event(event, batch) else {
        return 0;
    };
    let start = batch[..idx]
        .iter()
        .rposition(|e| breaks_zone_run(&e.event))
        .map_or(0, |pos| pos + 1);
    let end = batch[idx + 1..]
        .iter()
        .position(|e| breaks_zone_run(&e.event))
        .map_or(batch.len(), |pos| idx + 1 + pos);
    let run = &batch[start..end];
    let looks_back = changes_zone_all_looks_back(origin, destination);
    let entry_matches = |entry: &JournalEntry| -> bool {
        let GameEvent::ZoneChanged {
            object, from, to, ..
        } = entry.event
        else {
            return false;
        };
        if exclude_self && object == context.source {
            return false;
        }
        if origin.is_some_and(|want| !zone_ref_matches(want, from))
            || destination.is_some_and(|want| !zone_ref_matches(want, to))
        {
            return false;
        }
        if from == crate::zone::Zone::Battlefield && looks_back {
            return super::matching::departed_matches(filter, state, object, you, context);
        }
        let Some(current) = state.object_or_departed(object) else {
            return false;
        };
        let moves_after = batch
            .iter()
            .filter(|e| {
                (if looks_back {
                    e.seq >= entry.seq
                } else {
                    e.seq > entry.seq
                }) && matches!(e.event, GameEvent::ZoneChanged { object: id, .. } if id == object)
            })
            .count();
        let snapshot = if moves_after == 0 {
            current
        } else {
            let version = current
                .version
                .wrapping_sub(u32::try_from(moves_after).expect("zone changes fit version"));
            state
                .source_object(baylee_core::ids::DamageSourceRef { object, version })
                .unwrap_or(current)
        };
        crate::eval::matches_with_context(filter, state, snapshot, you, context)
    };

    let mut first_idx = None;
    let mut count = 0_u32;
    for (offset, candidate) in run.iter().enumerate() {
        if entry_matches(candidate) {
            if first_idx.is_none() {
                first_idx = Some(start + offset);
            }
            count = count.saturating_add(1);
        }
    }
    if first_idx == Some(idx) { count } else { 0 }
}

/// Count creatures declared attacking `attacked` in `batch` (`CR 508.3b`),
/// returning the count only on the first `BecameAttacker` targeting `attacked`.
pub(super) fn player_attacked_count(
    event: &GameEvent,
    batch: &[JournalEntry],
    attacked: PlayerId,
) -> u32 {
    let GameEvent::BecameAttacker {
        defending: baylee_core::ids::Defender::Player(target),
        ..
    } = *event
    else {
        return 0;
    };
    if target != attacked {
        return 0;
    }
    let Some(idx) = index_of_event(event, batch) else {
        return 0;
    };
    let mut first_idx = None;
    let mut count = 0_u32;
    for (i, entry) in batch.iter().enumerate() {
        if matches!(
            entry.event,
            GameEvent::BecameAttacker {
                defending: baylee_core::ids::Defender::Player(p),
                ..
            } if p == attacked
        ) {
            if first_idx.is_none() {
                first_idx = Some(i);
            }
            count = count.saturating_add(1);
        }
    }
    if first_idx == Some(idx) { count } else { 0 }
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
                tap_observation: None,
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
