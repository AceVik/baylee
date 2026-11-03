//! Foretell's special action and its printed-card offer (CR 702.143a).

use super::{CardLookup, Engine, EngineError, PlayerId};
use crate::choice::LegalActions;
use crate::event::Cause;
use crate::zone::{ZoneLocation, ZonePosition};
use baylee_core::ids::ObjectId;
use baylee_core::mana::ManaCost;

const FORETELL_ACTION_COST: ManaCost = ManaCost::parse("{2}");

impl<L: CardLookup> Engine<L> {
    /// Foretell any time its player has priority in their own turn
    /// (CR 702.143a): `foretellable` when the pool pays the {2}, and
    /// `unpaid_foretell` when it does not yet but a mana ability is on offer
    /// to make it with first (CR 605.3a). Called once the mana offers are in.
    pub(super) fn offer_foretell(&self, player: PlayerId, legal: &mut LegalActions) {
        if self.state.turn.active != player {
            return;
        }
        let makes_mana = self.offers_mana(legal);
        for &card in self.state.zones.list(ZoneLocation::Hand(player)) {
            let Some(source) = self.state.source_identity(card) else {
                continue;
            };
            let foretells = self
                .state
                .object(card)
                .and_then(|object| object.card)
                .and_then(|card| self.lookup.card(card.index))
                .is_some_and(|def| def.faces[0].foretell.is_some());
            if !foretells {
                continue;
            }
            if self.can_pay_mana(
                player,
                crate::casting::SpendFor::OtherFrom(source),
                &FORETELL_ACTION_COST,
            ) {
                legal.foretellable.push(card);
            } else if makes_mana {
                legal.unpaid_foretell.push(card);
            }
        }
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
