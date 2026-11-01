//! Foretell's special action and its printed-card offer (CR 702.143a).

use super::{CardLookup, Engine, EngineError, PlayerId};
use crate::choice::LegalActions;
use crate::event::Cause;
use crate::zone::{ZoneLocation, ZonePosition};
use baylee_core::ids::ObjectId;
use baylee_core::mana::ManaCost;

const FORETELL_ACTION_COST: ManaCost = ManaCost::parse("{2}");

impl<L: CardLookup> Engine<L> {
    pub(super) fn offer_foretell(&self, player: PlayerId, legal: &mut LegalActions) {
        if self.state.turn.active != player {
            return;
        }
        legal.foretellable = self
            .state
            .zones
            .list(ZoneLocation::Hand(player))
            .iter()
            .copied()
            .filter(|&card| {
                let Some(source) = self.state.source_identity(card) else {
                    return false;
                };
                self.state
                    .object(card)
                    .and_then(|object| object.card)
                    .and_then(|card| self.lookup.card(card.index))
                    .is_some_and(|def| def.faces[0].foretell.is_some())
                    && self.can_pay_mana(
                        player,
                        crate::casting::SpendFor::OtherFrom(source),
                        &FORETELL_ACTION_COST,
                    )
            })
            .collect();
    }

    pub(super) fn foretell(&mut self, player: PlayerId, card: ObjectId) -> Result<(), EngineError> {
        if self.state.turn.active != player {
            return Err(EngineError::IllegalAction("foretell requires your turn"));
        }
        let object = self
            .state
            .object(card)
            .filter(|object| object.zone == crate::zone::Zone::Hand)
            .filter(|object| object.zone_owner == Some(player))
            .ok_or(EngineError::IllegalAction("foretell requires your hand"))?;
        let has_foretell = object
            .card
            .and_then(|card| self.lookup.card(card.index))
            .is_some_and(|def| def.faces[0].foretell.is_some());
        if !has_foretell {
            return Err(EngineError::IllegalAction("not a foretell card"));
        }
        let origin = self
            .state
            .source_identity(card)
            .ok_or(EngineError::IllegalAction("missing cost source"))?;
        if !crate::casting::pay_mana_from(&mut self.state, player, origin, &FORETELL_ACTION_COST) {
            return Err(EngineError::IllegalAction("cannot pay foretell"));
        }
        let owner = self
            .state
            .object(card)
            .map_or(player, |object| object.owner);
        self.state.move_object(
            card,
            ZoneLocation::Exile(owner),
            ZonePosition::Top,
            Cause::Effect,
        )?;
        self.state
            .foretell_exiled_card(card, owner, self.state.turn.number);
        self.after_action(player);
        Ok(())
    }
}
