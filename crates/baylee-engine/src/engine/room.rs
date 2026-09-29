//! Rooms (CR 709.5): unlocking a door, the special action (CR 116.2m).
//!
//! What a door being locked *means* lives elsewhere: the rules text is
//! `CardDef::door_abilities`, read by `GameObject::printed_abilities`; the
//! name and mana cost are `GameState::set_doors`; the designation a Room
//! enters with is given where enter modifiers are (`apply_enter_modifiers`).
//! This is only the action, and the offer that lists it.
use super::{CardLookup, Engine, EngineError, GameEvent, ObjectId, Phase, PlayerId, Zone};
use baylee_core::mana::ManaCost;

impl<L: CardLookup> Engine<L> {
    /// What unlocking `half` of `id` costs: that half's mana cost (the
    /// "unlock cost", CR 709.5e), while `id` is a Room on the battlefield
    /// and that half is locked.
    fn unlock_cost(&self, id: ObjectId, half: u8) -> Option<ManaCost> {
        let o = self.state.object(id)?;
        if o.zone != Zone::Battlefield || !o.doors.is_room() || o.doors.is_unlocked(half) {
            return None;
        }
        self.lookup
            .card(o.card?.index)?
            .faces
            .get(usize::from(half))
            .map(|f| f.mana_cost)
    }

    /// The locked halves of `id` its controller could unlock with the mana
    /// floating now. What the offer lists, one slot per half, beside the
    /// sorcery timing it asks itself; the mana is floated first, as for
    /// turning a permanent face up.
    pub(super) fn unlockable_halves(&self, id: ObjectId) -> impl Iterator<Item = u8> + '_ {
        let pool = self
            .state
            .object(id)
            .filter(|o| o.doors.is_room())
            .map(|o| &self.state.players[o.controller.get() as usize].mana_pool);
        (0..2_u8).filter(move |&half| {
            pool.is_some_and(|pool| {
                self.unlock_cost(id, half)
                    .is_some_and(|cost| crate::mana_pay::can_pay(pool, &cost))
            })
        })
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
        if super::casting::pay_mana_for(
            &mut self.state,
            player,
            super::casting::SpendFor::Other,
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
