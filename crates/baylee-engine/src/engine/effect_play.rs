//! Casts made during a resolving instruction retain that instruction's continuation.

use super::{CardLookup, Engine, EngineError, ObjectId, PlayerId};
use crate::resolve::{AwaitingOp, Flow, Resolution};

/// A nested cast has not finished the spell or ability that instructed it.
#[derive(Clone, Debug)]
pub(super) struct EffectPlay {
    resolution: Box<Resolution>,
    control: Option<(PlayerId, PlayerId, ObjectId)>,
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
            .wrapping_add(crate::state::structural_fingerprint(&self.control))
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

    fn play_commanded_card(&mut self, player: PlayerId, card: ObjectId) -> Result<(), EngineError> {
        self.start_cast_wizard(player, card)
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
        parent.resolution.awaiting = None;
        parent.resolution.pc += 1;
        self.awaiting_answer = false;
        match crate::resolve::run(&mut self.state, &mut parent.resolution) {
            Flow::Complete => self.finish_resolution(&parent.resolution),
            Flow::Wait(pending) => {
                self.resolution = Some(*parent.resolution);
                self.pending = pending;
                self.awaiting_answer = true;
            }
        }
        true
    }
}
