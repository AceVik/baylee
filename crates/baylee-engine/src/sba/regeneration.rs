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

/// Both resolved prohibitions and live static abilities prevent regeneration.
pub(super) fn allowed(state: &GameState, id: baylee_core::ids::ObjectId) -> bool {
    state.object(id).is_none_or(|object| {
        !state
            .per_turn
            .cant_regenerate
            .contains(&(id, object.version))
            && (!state.effects.may_prevent_regeneration()
                || !state.effects.iter().any(|effect| {
                    effect.modifier == baylee_cards_dsl::Modifier::CantBeRegenerated
                        && crate::effects::applies_to(state, effect, object)
                }))
    })
}

/// Snapshot permissions before any member of a simultaneous event departs.
/// The ordinary case has no prohibitions and performs no battlefield scan.
pub(super) fn prohibited_on_battlefield(
    state: &GameState,
) -> smallvec::SmallVec<[baylee_core::ids::ObjectId; 8]> {
    if state.per_turn.cant_regenerate.is_empty() && !state.effects.may_prevent_regeneration() {
        return smallvec::SmallVec::new();
    }
    state
        .battlefield_seen()
        .filter(|id| !allowed(state, *id))
        .collect()
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
    #[test]
    fn regeneration_removes_incoming_combat_only_when_it_removes_the_creature() {
        use crate::{
            combat::{AttackerInfo, active_damage_recipient, remember_attack},
            engine::synthetic::{SyntheticLookup, preset},
            event::DamageTarget,
            object::ObjectKind,
            state::GameState,
            zone::ZoneLocation,
        };
        use baylee_core::{
            ids::{Defender, PlayerId},
            types::TypeSet,
        };
        for (attacking, blocking) in [(false, false), (true, false), (false, true)] {
            let mut state =
                GameState::from_preset(&preset(601, &[]), &SyntheticLookup::new(vec![])).unwrap();
            let name = state.names.intern("regenerating planeswalker creature");
            let target = state.create_bare(
                PlayerId::new(0),
                ObjectKind::Permanent,
                name,
                ZoneLocation::Battlefield,
            );
            let incoming = state.create_bare(
                PlayerId::new(1),
                ObjectKind::Permanent,
                name,
                ZoneLocation::Battlefield,
            );
            for id in [target, incoming] {
                let base = state.object_mut(id).unwrap().base_mut();
                base.types = if id == target {
                    TypeSet::CREATURE.union(TypeSet::PLANESWALKER)
                } else {
                    TypeSet::CREATURE
                };
                base.power = Some(3);
                base.toughness = Some(3);
            }
            state.invalidate_projections();
            state.refresh_characteristics();
            let target_ref = state.source_identity(target).unwrap();
            let incoming_ref = state.source_identity(incoming).unwrap();
            state.combat.declare_attackers([AttackerInfo {
                creature: incoming,
                defending: Defender::Planeswalker(target),
                blocked: false,
                band: None,
            }]);
            remember_attack(&mut state, incoming, Defender::Planeswalker(target));
            if attacking {
                state.combat.declare_attackers([AttackerInfo {
                    creature: target,
                    defending: Defender::Player(PlayerId::new(1)),
                    blocked: false,
                    band: None,
                }]);
                remember_attack(&mut state, target, Defender::Player(PlayerId::new(1)));
            }
            if blocking {
                state.combat.declare_block(target_ref, incoming_ref);
            }
            assert!(state.combat.is_being_attacked(target_ref));
            state.object_mut(target).unwrap().regeneration_shields = 1;
            super::super::destroy(&mut state, target);
            assert_eq!(
                state.combat.is_being_attacked(target_ref),
                !attacking && !blocking
            );
            assert_eq!(
                active_damage_recipient(&state, incoming),
                if attacking || blocking {
                    None
                } else {
                    Some(DamageTarget::Object(target))
                }
            );
            assert!(
                state.combat.is_attacking(incoming),
                "removing its defender does not remove the attacker"
            );
            assert!(!state.combat.is_attacking(target));
            assert!(!state.combat.is_blocking(target, incoming));
            assert_eq!(state.object(target).unwrap().regeneration_shields, 0);
        }
    }
}
