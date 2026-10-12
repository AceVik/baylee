//! Competing regeneration replacements, ordered once per simultaneous event.
use crate::choice::{Pending, YesNoPrompt};
use crate::state::GameState;
use baylee_core::ids::DamageSourceRef;
use std::collections::VecDeque;

/// Ordinary games pay one empty pointer; only competing replacements allocate.
#[derive(Clone, Debug, Default, Hash)]
pub(crate) struct RegenerationChoices(Option<Box<Queue>>);

#[derive(Clone, Debug, Default, Hash)]
struct Queue {
    entries: VecDeque<(usize, DamageSourceRef)>,
    ordered: bool,
}

impl RegenerationChoices {
    pub(crate) fn push(&mut self, reference: DamageSourceRef, order: usize) {
        let queue = self.0.get_or_insert_with(Box::default);
        queue.entries.push_back((order, reference));
        queue.ordered = false;
    }

    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_none()
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.0.as_ref().map_or(0, |queue| queue.entries.len())
    }

    fn front(&mut self) -> Option<DamageSourceRef> {
        let queue = self.0.as_mut()?;
        if !queue.ordered {
            // Stable APNAP ordering once, then O(1) consumption per answer.
            queue
                .entries
                .make_contiguous()
                .sort_by_key(|(order, _)| *order);
            queue.ordered = true;
        }
        queue.entries.front().map(|(_, reference)| *reference)
    }

    fn pop(&mut self) -> Option<DamageSourceRef> {
        let queue = self.0.as_mut()?;
        let reference = queue.entries.pop_front().map(|(_, reference)| reference);
        if queue.entries.is_empty() {
            self.0 = None;
        }
        reference
    }
}

/// Both replacements perform identical regeneration actions. Decide which shield
/// is consumed before the next instruction or SBA can observe the result.
pub(crate) fn regeneration_choice(state: &mut GameState) -> Option<Pending> {
    while let Some(reference) = state.regeneration_choices.front() {
        if let Some(object) = state.object(reference.object)
            && object.version == reference.version
            && object.regeneration_shields > 0
            && !state.has_left(object.controller)
        {
            return Some(Pending::YesNo {
                player: object.controller,
                prompt: YesNoPrompt::UseRegenerationShield {
                    permanent: object.id,
                },
                source: None,
            });
        }
        state.regeneration_choices.pop();
    }
    None
}

pub(crate) fn answer_regeneration_choice(state: &mut GameState, use_shield: bool) {
    let reference = state
        .regeneration_choices
        .pop()
        .expect("offered regeneration choice");
    if use_shield
        && let Some(object) = state.object_mut(reference.object)
        && object.version == reference.version
    {
        object.regeneration_shields = object.regeneration_shields.saturating_sub(1);
    }
}

#[cfg(test)]
mod tests {
    use super::RegenerationChoices;
    use baylee_core::ids::{DamageSourceRef, ObjectId};

    #[test]
    fn large_queue_keeps_apnap_and_stable_order_then_releases_storage() {
        let mut queue = RegenerationChoices::default();
        for round in 0..3 {
            for slot in 0..4096_u32 {
                queue.push(
                    DamageSourceRef {
                        object: ObjectId::new(slot, 0),
                        version: round,
                    },
                    (slot % 4) as usize,
                );
            }
            // Independent reference: scan the original sequence once per seat.
            for seat in 0..4 {
                for slot in (seat..4096).step_by(4) {
                    let expected = DamageSourceRef {
                        object: ObjectId::new(slot, 0),
                        version: round,
                    };
                    assert_eq!(queue.front(), Some(expected));
                    assert_eq!(queue.front(), Some(expected));
                    assert_eq!(queue.pop(), Some(expected));
                }
            }
            assert!(queue.is_empty());
            assert_eq!(queue.front(), None);
            assert_eq!(queue.pop(), None);
        }
    }
}
