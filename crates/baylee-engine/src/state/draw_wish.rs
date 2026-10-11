//! Ring of Ma'rûf replaces one draw, even from an empty library (CR 614.11).
use super::{
    GameState, ObjectId, PlayerId, ZoneLocation, ZonePosition, draw_offer::DrawReplacement,
};
use crate::choice::{ChoicePrompt, Pending};
use baylee_cards_dsl::Modifier;
use baylee_core::ids::EffectId;

impl GameState {
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

    /// Consumes exactly one replacement, without recording a draw or revealing the card.
    pub(crate) fn draw_wish_answered(
        &mut self,
        replacement: DrawReplacement,
        chosen: Option<ObjectId>,
    ) -> Option<(PlayerId, DrawReplacement)> {
        let DrawReplacement::Wish(effect) = replacement else {
            return self.next_draw_offer();
        };
        let &(player, _) = self.draws_to_offer.front()?;
        if self.effects.iter().any(|entry| {
            entry.id == effect
                && entry.controller == player
                && entry.modifier == Modifier::NextDrawWish
        }) {
            self.effects.remove_where(|entry| entry.id == effect);
            if let Some(card) = chosen.filter(|card| self.draw_wish_options(player).contains(card))
            {
                let _ = self.move_object(
                    card,
                    ZoneLocation::Hand(player),
                    ZonePosition::Top,
                    crate::event::Cause::Effect,
                );
            }
            self.take_front_draw();
        }
        self.next_draw_offer()
    }
}
