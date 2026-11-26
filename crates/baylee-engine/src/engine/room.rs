//! Rooms (CR 709.5): unlocking a door, the special action (CR 116.2m).
//!
//! What a door being locked *means* lives elsewhere: the rules text is
//! `CardDef::door_abilities`, read by `GameObject::printed_abilities`; the
//! name and mana cost are `GameState::set_doors`; the designation a Room
//! enters with is given where enter modifiers are (`apply_enter_modifiers`).
//! This is only the action, and the offer that lists it.
use super::{CardLookup, Engine, EngineError, GameEvent, ObjectId, Phase, PlayerId, Zone};
use baylee_core::mana::ManaCost;

impl<L: CardLookup, D: crate::engine::LoopDetection> Engine<L, D> {
    /// What unlocking `half` of `id` costs: that half's mana cost (the
    /// "unlock cost", CR 709.5e), while `id` is a Room on the battlefield
    /// and that half is locked.
    pub(super) fn unlock_cost(&self, id: ObjectId, half: u8) -> Option<ManaCost> {
        let o = self.state.object(id)?;
        if o.zone != Zone::Battlefield || !o.doors.is_room() || o.doors.is_unlocked(half) {
            return None;
        }
        let base = self
            .lookup
            .card(o.card?.index)?
            .faces
            .get(usize::from(half))
            .map(|f| f.mana_cost)?;
        let mut reduction = 0_u8;
        for fx in self.state.effects.iter() {
            if let baylee_cards_dsl::Modifier::UnlockCostsLess { who, amount } = fx.modifier
                && crate::eval::players_for_source(who, &self.state, fx.controller, fx.source)
                    .is_some_and(|ps| ps.contains(&o.controller))
            {
                reduction = reduction.saturating_add(amount);
            }
        }
        Some(base.with_less_generic(u32::from(reduction)))
    }

    /// CR 709.5e: the controller pays a locked half's mana cost, and the
    /// permanent is given that half's unlocked designation. A special action
    /// (CR 116.2m): no stack, and taken any time its controller has priority
    /// with the stack empty in a main phase of their own turn. The unlock is
    /// journalled, which is what "when you unlock this door" hears
    /// (CR 709.5h).
    pub(super) fn unlock_door(
        &mut self,
        player: PlayerId,
        id: ObjectId,
        half: u8,
    ) -> Result<(), EngineError> {
        let cost = self
            .unlock_cost(id, half)
            .ok_or(EngineError::IllegalAction("no locked door to unlock"))?;
        let (controller, card, unlocked) = self
            .state
            .object(id)
            .and_then(|o| Some((o.controller, o.card?, o.doors.unlocked())))
            .ok_or(EngineError::IllegalAction("no such Room"))?;
        if controller != player {
            return Err(EngineError::IllegalAction("not your permanent"));
        }
        let main = matches!(self.state.turn.phase, Phase::FirstMain | Phase::SecondMain);
        if !main || self.state.turn.active != player || !self.state.zones.stack_is_empty() {
            return Err(EngineError::IllegalAction(
                "a door is unlocked only as a sorcery",
            ));
        }
        let def = self
            .lookup
            .card(card.index)
            .ok_or(EngineError::IllegalAction("no such card"))?;
        let origin = self
            .state
            .source_identity(id)
            .ok_or(EngineError::IllegalAction("missing cost source"))?;
        if super::casting::pay_mana_for(
            &mut self.state,
            player,
            super::casting::SpendFor::OtherFrom(origin),
            &cost,
        )
        .is_none()
        {
            return Err(EngineError::IllegalAction("cannot pay the unlock cost"));
        }
        self.state.set_doors(id, def, unlocked | (1 << half));
        self.state
            .journal
            .record(GameEvent::DoorUnlocked { object: id, half });
        self.after_action(player);
        Ok(())
    }
}
