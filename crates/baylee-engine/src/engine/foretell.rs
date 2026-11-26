//! Foretell's special action and its printed-card offer (CR 702.143a).

use super::{CardLookup, Engine, EngineError, PlayerId};
use crate::choice::LegalActions;
use crate::event::Cause;
use crate::zone::{ZoneLocation, ZonePosition};
use baylee_core::ids::ObjectId;
use baylee_core::mana::ManaCost;

const FORETELL_ACTION_COST: ManaCost = ManaCost::parse("{2}");

impl<L: CardLookup, D: crate::engine::LoopDetection> Engine<L, D> {
    fn may_foretell_now(&self, player: PlayerId) -> bool {
        if self.state.turn.active == player {
            return true;
        }
        self.state.effects.iter().any(|fx| match fx.modifier {
            baylee_cards_dsl::Modifier::MayForetellAnyTurn { who } => {
                crate::eval::players_for_source(who, &self.state, fx.controller, fx.source)
                    .is_some_and(|ps| ps.contains(&player))
            }
            _ => false,
        })
    }

    fn foretell_action_cost(&self, player: PlayerId) -> ManaCost {
        let mut reduction = 0_u8;
        for fx in self.state.effects.iter() {
            if let baylee_cards_dsl::Modifier::ForetellActionCostsLess {
                who,
                amount,
                first_each_turn_only,
            } = fx.modifier
            {
                if first_each_turn_only && self.state.per_turn.cards_foretold_by(player) > 0 {
                    continue;
                }
                if crate::eval::players_for_source(who, &self.state, fx.controller, fx.source)
                    .is_some_and(|ps| ps.contains(&player))
                {
                    reduction = reduction.saturating_add(amount);
                }
            }
        }
        FORETELL_ACTION_COST.with_less_generic(u32::from(reduction))
    }

    /// Foretell any time its player has priority in their own turn (or any
    /// turn under *Cosmos Charger*, CR 702.143a): `foretellable` when the pool
    /// pays the {2} (adjusted by active reductions like *Cosmos Charger* or
    /// *Ranar*), and `unpaid_foretell` when it does not yet but a mana ability
    /// is on offer to make it with first (CR 605.3a). Called once the mana
    /// offers are in.
    pub(super) fn offer_foretell(&self, player: PlayerId, legal: &mut LegalActions) {
        if !self.may_foretell_now(player) {
            return;
        }
        let action_cost = self.foretell_action_cost(player);
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
                &action_cost,
            ) {
                legal.foretellable.push(card);
            } else if makes_mana {
                legal.unpaid_foretell.push(card);
            }
        }
    }

    pub(super) fn foretell(&mut self, player: PlayerId, card: ObjectId) -> Result<(), EngineError> {
        if !self.may_foretell_now(player) {
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
        let action_cost = self.foretell_action_cost(player);
        if !crate::casting::pay_mana_from(&mut self.state, player, origin, &action_cost) {
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
        self.state.per_turn.note_foretell(player);
        self.after_action(player);
        Ok(())
    }
}
