//! Plot's special action and its printed-card offer (`CR 702.170a`, `CR 116.2k`).
//!
//! Any time a player has priority during their main phase while the stack is
//! empty, they may exile a card with plot from their hand and pay its plot
//! cost. That card becomes plotted (`Rider::Plotted` + `Rider::PlottedOnTurn`),
//! and on any later turn its owner may cast it from exile without paying its
//! mana cost during their main phase while the stack is empty (`CR 702.170d`).

use super::{CardLookup, Engine, EngineError, PlayerId};
use crate::choice::{LegalActions, PLOT_CARD};
use crate::zone::ZoneLocation;
use baylee_core::ids::ObjectId;
use baylee_core::mana::ManaCost;

impl<L: CardLookup, D: crate::engine::LoopDetection> Engine<L, D> {
    /// Printed plot cost of `card` in `player`'s hand when `player` is in their
    /// own main phase with an empty stack (`CR 702.170a`).
    pub(super) fn plot_cost(&self, player: PlayerId, card: ObjectId) -> Option<ManaCost> {
        if !crate::casting::sorcery_window(&self.state, player) {
            return None;
        }
        let object = self
            .state
            .object(card)
            .filter(|o| o.zone == crate::zone::Zone::Hand && o.zone_owner == Some(player))?;
        let mut cost = self.lookup.card(object.card?.index)?.faces[0].plot?;
        for effect in self.state.effects.iter() {
            let baylee_cards_dsl::Modifier::PlotActionCostsLess { who, amount } = effect.modifier
            else {
                continue;
            };
            if amount > 0
                && crate::effects::source_is_present(&self.state, effect)
                && crate::eval::players_for_source(
                    who,
                    &self.state,
                    effect.controller,
                    effect.source,
                )
                .is_some_and(|players| players.contains(&player))
            {
                cost = cost.with_less_generic(u32::from(amount));
            }
        }
        Some(cost)
    }

    /// Offers `PLOT_CARD` in `legal.abilities` (when currently payable) or in
    /// `legal.unpaid_abilities` (when mana sources are available to make the
    /// plot cost first). Called once all mana offers have been collected.
    pub(super) fn offer_plot(&self, player: PlayerId, legal: &mut LegalActions) {
        if !crate::casting::sorcery_window(&self.state, player) {
            return;
        }
        let makes_mana = self.offers_mana(legal);
        for &card in self.state.zones.list(ZoneLocation::Hand(player)) {
            let Some(cost) = self.plot_cost(player, card) else {
                continue;
            };
            let Some(origin) = self.state.source_identity(card) else {
                continue;
            };
            if self.can_pay_mana(
                player,
                crate::casting::SpendFor::OtherFrom(origin),
                &cost.with_x(0),
            ) {
                legal.abilities.push((card, PLOT_CARD));
            } else if makes_mana && cost != ManaCost::ZERO {
                legal.abilities.push((card, PLOT_CARD));
                legal.unpaid_abilities.push((card, PLOT_CARD, cost));
            }
        }
    }

    /// Executes the `PLOT_CARD` special action (`CR 702.170a`, `CR 116.2k`).
    pub(super) fn plot_card(
        &mut self,
        player: PlayerId,
        card: ObjectId,
    ) -> Result<(), EngineError> {
        if !crate::casting::sorcery_window(&self.state, player) {
            return Err(EngineError::IllegalAction(
                "a card is plotted only as a sorcery",
            ));
        }
        let cost = self
            .plot_cost(player, card)
            .ok_or(EngineError::IllegalAction("not a plot card in your hand"))?;
        let origin = self
            .state
            .source_identity(card)
            .ok_or(EngineError::IllegalAction("missing cost source"))?;
        if !crate::casting::pay_mana_from(&mut self.state, player, origin, &cost.with_x(0)) {
            return Err(EngineError::IllegalAction("cannot pay the plot cost"));
        }
        self.state.plot_card_into_exile(player, card);
        self.after_action(player);
        Ok(())
    }
}
