//! Casts made during a resolving instruction retain that instruction's continuation.

use super::{CardLookup, Engine, EngineError, ObjectId, PlayerId};
use crate::resolve::{AwaitingOp, Flow, Resolution};

/// A nested cast has not finished the spell or ability that instructed it.
#[derive(Clone, Debug)]
pub(super) struct EffectPlay {
    resolution: Box<Resolution>,
    control: Option<(PlayerId, PlayerId, ObjectId)>,
    linked: Option<LinkedPlay>,
    awaiting_land_entry: bool,
}

#[derive(Clone, Copy, Debug, Hash)]
struct LinkedPlay {
    card: baylee_core::ids::DamageSourceRef,
    face_down: bool,
    opened: bool,
}

impl EffectPlay {
    pub(super) fn control(&self) -> Option<(PlayerId, PlayerId)> {
        self.control
            .map(|(controller, player, _)| (controller, player))
    }
    pub(super) fn commanded_card(&self) -> Option<ObjectId> {
        self.control.map(|(_, _, card)| card)
    }
    pub(super) fn fingerprint(&self) -> u64 {
        self.resolution
            .program_fingerprint()
            .wrapping_mul(31)
            .wrapping_add(
                self.resolution
                    .awaiting
                    .as_ref()
                    .map_or(0, AwaitingOp::search_fingerprint),
            )
            .wrapping_mul(31)
            .wrapping_add(crate::state::structural_fingerprint(&(
                self.control,
                self.linked,
                self.awaiting_land_entry,
            )))
    }
}

impl<L: CardLookup> Engine<L> {
    pub(super) fn answer_effect_play(
        &mut self,
        player: PlayerId,
        cards: &[ObjectId],
    ) -> Option<Result<(), EngineError>> {
        let awaiting = self.resolution.as_ref()?.awaiting.as_ref()?;
        let subject = match awaiting {
            AwaitingOp::LinkedPlay { .. } => {
                self.answer_linked_play(player, cards);
                return Some(Ok(()));
            }
            AwaitingOp::MaskedCast => None,
            AwaitingOp::ControlledCard { player } => Some(*player),
            _ => return None,
        };
        let resolution = self.resolution.take()?;
        let control = subject
            .zip(cards.first().copied())
            .map(|(subject, card)| (resolution.controller, subject, card));
        if let Some((_, subject, card)) = control
            && let Some(card) = self.state.source_identity(card)
        {
            self.state
                .constrained_payments
                .push(crate::constrained_payment::ConstrainedPayment {
                    player: subject,
                    card,
                    required: baylee_core::mana::ManaPool::new(),
                });
        }
        self.effect_plays.push(EffectPlay {
            resolution: Box::new(resolution),
            control,
            linked: None,
            awaiting_land_entry: false,
        });
        let result = cards.first().map_or(
            Err(EngineError::IllegalAction("casting declined")),
            |&card| {
                if let Some(subject) = subject {
                    self.play_commanded_card(subject, card)
                } else {
                    self.start_masked_cast(player, card)
                }
            },
        );
        if result.is_err() {
            self.finish_nested_cast();
        }
        Some(Ok(()))
    }

    fn answer_linked_play(&mut self, player: PlayerId, cards: &[ObjectId]) {
        let mut resolution = self.resolution.take().expect("linked play resolution");
        let Some(AwaitingOp::LinkedPlay { cards: remaining }) = resolution.awaiting.as_mut() else {
            unreachable!("linked play continuation")
        };
        let chosen = cards
            .first()
            .and_then(|id| remaining.iter().find(|r| r.object == *id))
            .copied();
        if let Some(chosen) = chosen {
            remaining.retain(|r| *r != chosen);
        } else {
            remaining.clear();
        }
        let linked = chosen
            .filter(|r| {
                self.state
                    .object(r.object)
                    .is_some_and(|o| o.zone == crate::zone::Zone::Exile && o.version == r.version)
            })
            .map(|card| {
                let face_down = self.state.reveal_exiled_card(card.object);
                let object = self.state.object_mut(card.object).expect("chosen exile");
                let rider = crate::object::Rider::PlayableFromExileFor(player);
                let opened = !object.riders.contains(&rider);
                if opened {
                    object.riders.push(rider);
                }
                LinkedPlay {
                    card,
                    face_down,
                    opened,
                }
            });
        self.effect_plays.push(EffectPlay {
            resolution: Box::new(resolution),
            control: None,
            linked,
            awaiting_land_entry: false,
        });
        if let Some(linked) = linked {
            if self
                .start_linked_free_play(player, linked.card.object)
                .is_err()
            {
                self.finish_nested_cast();
            }
        } else {
            self.finish_nested_cast();
        }
    }

    fn play_commanded_card(&mut self, player: PlayerId, card: ObjectId) -> Result<(), EngineError> {
        self.start_cast_wizard(player, card)
    }

    /// A land play is not complete until its entry replacements have finished.
    pub(super) fn suspend_nested_land_entry(&mut self) {
        if let Some(parent) = self.effect_plays.last_mut() {
            parent.awaiting_land_entry = true;
        }
    }

    pub(super) fn resume_nested_land_entry(&mut self) -> bool {
        if self
            .effect_plays
            .last()
            .is_some_and(|parent| parent.awaiting_land_entry)
        {
            self.finish_nested_cast();
            true
        } else {
            false
        }
    }

    /// Called before ordinary priority/SBA processing when the nested cast ends.
    pub(super) fn finish_nested_cast(&mut self) -> bool {
        let Some(mut parent) = self.effect_plays.pop() else {
            return false;
        };
        if parent.control.is_some() {
            self.state.constrained_payments.pop();
        }
        if let Some((controller, player, card)) = parent.control
            && self
                .state
                .object(card)
                .is_some_and(|object| object.zone == crate::zone::Zone::Stack)
        {
            self.remember_controlled_spell(card, controller, player);
        }
        if let Some(linked) = parent.linked
            && self.state.object(linked.card.object).is_some_and(|o| {
                o.zone == crate::zone::Zone::Exile && o.version == linked.card.version
            })
        {
            if linked.face_down {
                self.state.conceal_exiled_card(linked.card.object);
            }
            if linked.opened
                && let Some(object) = self.state.object_mut(linked.card.object)
            {
                let rider =
                    crate::object::Rider::PlayableFromExileFor(parent.resolution.controller);
                object.riders.retain(|r| *r != rider);
            }
        }
        if let Some(AwaitingOp::LinkedPlay { cards }) = parent.resolution.awaiting.take()
            && let Some(question) = crate::resolve::hidden_exile::continue_play(
                &self.state,
                &mut parent.resolution,
                cards,
            )
        {
            self.resolution = Some(*parent.resolution);
            self.pending = question;
            self.awaiting_answer = true;
            return true;
        }
        parent.resolution.awaiting = None;
        parent.resolution.pc += 1;
        self.awaiting_answer = false;
        match crate::resolve::run(&mut self.state, &mut parent.resolution) {
            Flow::Complete => self.finish_resolution(&parent.resolution),
            Flow::Enter => {
                self.resolution = Some(*parent.resolution);
                self.awaiting_answer = false;
            }
            Flow::Wait(pending) => {
                self.resolution = Some(*parent.resolution);
                self.pending = pending;
                self.awaiting_answer = true;
            }
        }
        true
    }
}
