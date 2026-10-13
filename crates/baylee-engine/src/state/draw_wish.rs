//! Ring of Ma'rûf replaces one draw, even from an empty library (CR 614.11).
use super::{GameState, ObjectId, PlayerId, ZoneLocation, ZonePosition};
use crate::choice::{ChoicePrompt, Pending};
use baylee_cards_dsl::Modifier;
use baylee_core::ids::EffectId;

impl GameState {
    #[cfg(test)]
    pub(crate) fn draw_wish_effect(
        &self,
        player: PlayerId,
        source: Option<ObjectId>,
    ) -> Option<(EffectId, ObjectId)> {
        self.effects.iter().find_map(|effect| {
            if effect.controller != player || effect.modifier != Modifier::NextDrawWish {
                return None;
            }
            let origin = effect.source?;
            source
                .is_none_or(|source| source == origin)
                .then_some((effect.id, origin))
        })
    }

    pub(crate) fn draw_wish_options(&self, player: PlayerId) -> Vec<ObjectId> {
        // This ability does not specifically authorize an outside card in Commander (CR 903.11).
        if self.format == baylee_core::preset::FormatId::Commander {
            return Vec::new();
        }
        self.zones
            .list(ZoneLocation::OutsideGame(player))
            .iter()
            .copied()
            .filter(|id| {
                self.object(*id)
                    .is_some_and(|object| object.owner == player && object.is_card())
            })
            .collect()
    }

    pub(crate) fn draw_wish_question(&self, player: PlayerId) -> Pending {
        let options = self.draw_wish_options(player);
        Pending::ChooseCards {
            player,
            min: u8::from(!options.is_empty()),
            max: 1,
            options,
            prompt: ChoicePrompt::Wish,
            total: None,
        }
    }

    /// Consumes exactly one replacement, without recording a draw or
    /// revealing the card: the front draw is replaced by `chosen`, if it is
    /// still a card `player` owns outside the game.
    pub(crate) fn draw_wish_answered(
        &mut self,
        player: PlayerId,
        effect: EffectId,
        chosen: Option<ObjectId>,
    ) {
        if !self.effects.iter().any(|entry| {
            entry.id == effect
                && entry.controller == player
                && entry.modifier == Modifier::NextDrawWish
        }) {
            return;
        }
        self.effects.remove_where(|entry| entry.id == effect);
        if let Some(card) = chosen.filter(|card| self.draw_wish_options(player).contains(card)) {
            let _ = self.move_object(
                card,
                ZoneLocation::Hand(player),
                ZonePosition::Top,
                crate::event::Cause::Effect,
            );
        }
        self.take_front_draw();
    }
}

impl GameState {
    /// The card `player` drew last this turn, while it is still in their hand
    /// as the object it was drawn as (Jandor's Ring's "the last card you drew
    /// this turn"). A card drawn later and kept is what counts; one drawn
    /// last and gone means there is nothing to discard, not the one before.
    #[must_use]
    pub(crate) fn last_drawn_in_hand(&self, player: PlayerId) -> Option<ObjectId> {
        let &(id, version) = self
            .per_turn
            .drawn
            .iter()
            .rev()
            .find(|(id, _)| self.object(*id).is_some_and(|o| o.owner == player))?;
        self.object(id)
            .filter(|o| o.version == version && o.zone == super::Zone::Hand)
            .map(|_| id)
    }
}
