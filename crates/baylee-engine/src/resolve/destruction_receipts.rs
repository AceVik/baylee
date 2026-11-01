//! Simultaneous destruction followed by one instruction per actual destruction.

use super::Resolution;
use crate::{
    event::{Cause, GameEvent},
    state::GameState,
    zone::Zone,
};
use baylee_cards_dsl::{Effect, Filter};
use baylee_core::ids::{DamageSourceRef, ObjectId, PlayerId};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct Receipt {
    identity: DamageSourceRef,
    controller: PlayerId,
    power: i16,
}

/// The captured battlefield identities survive replacement and payment questions.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct Batch {
    operation: Effect,
    since: usize,
    remaining: Vec<Receipt>,
    collected: bool,
    previous_object: Option<ObjectId>,
    previous_player: Option<PlayerId>,
    previous_identity: Option<(DamageSourceRef, i16)>,
}

pub(super) fn execute(
    state: &mut GameState,
    res: &mut Resolution,
    filter: &'static Filter,
    no_regen: bool,
    effects: &'static [Effect],
    operation: Effect,
) {
    if res
        .subject
        .destruction_batches
        .last()
        .is_none_or(|batch| batch.operation != operation)
    {
        let remaining: Vec<_> = state
            .battlefield_seen()
            .filter_map(|id| {
                let object = state.object(id)?;
                crate::eval::matches_with_context(
                    filter,
                    state,
                    object,
                    res.controller,
                    res.rule_context(),
                )
                .then_some(Receipt {
                    identity: DamageSourceRef {
                        object: id,
                        version: object.version,
                    },
                    controller: object.controller,
                    power: object.characteristics().power.unwrap_or(0),
                })
            })
            .collect();
        let objects: Vec<_> = remaining
            .iter()
            .map(|receipt| receipt.identity.object)
            .collect();
        let previous_player = res.subject.event_player();
        let previous_identity = res.subject.event_identity();
        res.subject.destruction_batches.push(Batch {
            operation,
            since: state.journal.entries().len(),
            remaining,
            collected: false,
            previous_object: res.event_object,
            previous_player,
            previous_identity,
        });
        crate::sba::destroy_all(state, &objects, no_regen);
        // The resolution driver settles all destruction replacement questions
        // before re-entering this operation or executing any child instruction.
        res.effects.insert(res.pc + 1, operation);
        return;
    }
    let batch = res
        .subject
        .destruction_batches
        .last_mut()
        .expect("batch above");
    if !batch.collected {
        batch.remaining.retain(|receipt| {
            state.journal.entries()[batch.since..].iter().any(|entry| {
                matches!(entry.event, GameEvent::ZoneChanged {
                object, from: Zone::Battlefield, cause: Cause::Destroy, ..
            } if object == receipt.identity.object)
                    && entry
                        .departure
                        .as_ref()
                        .is_some_and(|departure| departure.version == receipt.identity.version)
            })
        });
        // Arena traversal is deterministic. Each permanent contributes its
        // own choice, even when several had the same controller.
        batch.remaining.reverse();
        batch.collected = true;
    }
    if let Some(receipt) = batch.remaining.pop() {
        res.event_object = Some(receipt.identity.object);
        res.subject.set_event_player(Some(receipt.controller));
        res.subject
            .set_event_identity(Some((receipt.identity, receipt.power)));
        let mut program = super::flatten(effects);
        program.push(operation);
        let insertion = res.pc + 1;
        res.effects.splice(insertion..insertion, program);
    } else {
        let batch = res.subject.destruction_batches.pop().expect("batch above");
        res.event_object = batch.previous_object;
        res.subject.set_event_player(batch.previous_player);
        res.subject.set_event_identity(batch.previous_identity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_suspended_destruction_loop_hashes_original_controller_and_incarnation() {
        let identity = DamageSourceRef {
            object: ObjectId::new(2, 0),
            version: 7,
        };
        let mut original = super::super::SubjectContext::default();
        original.destruction_batches.push(Batch {
            operation: Effect::DestroyAllThenForEachDestroyed {
                filter: &Filter::LAND,
                no_regen: false,
                effects: &[],
            },
            since: 4,
            remaining: vec![Receipt {
                identity,
                controller: PlayerId::new(0),
                power: 2,
            }],
            collected: true,
            previous_object: None,
            previous_player: None,
            previous_identity: None,
        });
        let mut other = original.clone();
        other.destruction_batches[0].remaining[0].controller = PlayerId::new(1);
        assert_ne!(original.fingerprint(), other.fingerprint());
        other = original.clone();
        other.destruction_batches[0].remaining[0].identity.version += 1;
        assert_ne!(original.fingerprint(), other.fingerprint());
        other = original.clone();
        other.destruction_batches[0].collected = false;
        assert_ne!(original.fingerprint(), other.fingerprint());
    }
}
