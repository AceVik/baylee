//! Island Sanctuary's skip, offered for every draw its controller would
//! make during their draw step.
//!
//! "If you would draw a card during your draw step, instead you may skip
//! that draw." A skip is a replacement effect (CR 614.10), and a draw is an
//! event of its own however many an instruction asks for: "If a player is
//! instructed to draw multiple cards, that player performs that many
//! individual card draws" (CR 121.2). So the turn-based draw (CR 504.1), a
//! Howling Mine's additional card and an Ancestral Recall cast in that step
//! are each offered the skip, card by card, and a replaced draw in a
//! sequence is finished before the sequence goes on (CR 614.11a). The skip
//! is offered even to a player with no library (CR 614.11).
//!
//! `GameState::draw_cards` cannot ask, so a draw that could be skipped
//! waits in [`GameState::draws_to_offer`]. Two drainers ask about it: a
//! resolution, before its next instruction (`resolve::run`), and the
//! machine, for the turn-based draw and anything queued outside a
//! resolution (`Engine::offer_queued_draw`). Both build the same question
//! here and hand the answer back here.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary
use super::*;
use crate::choice::{Pending, YesNoPrompt};
use crate::object::Status;
use baylee_cards_dsl::{AbilityDef, ReplacementRule};

impl GameState {
    /// The first Island Sanctuary `player` controls that could replace a
    /// draw of theirs now and is not in `declined`: only during their own
    /// draw step (CR 504), only while they are in the game, and only from
    /// a source on the battlefield that has not phased out.
    pub(crate) fn draw_skip_source(
        &self,
        player: PlayerId,
        declined: &[ObjectId],
    ) -> Option<ObjectId> {
        if self.turn.active != player
            || self.turn.step != crate::turn::Step::Draw
            || self.has_left(player)
        {
            return None;
        }
        self.replacement_rules
            .iter()
            .filter(|r| r.rule == ReplacementRule::MaySkipDrawStepDraw && r.controller == player)
            .map(|r| r.source)
            .filter(|s| !declined.contains(s))
            .find(|s| {
                self.object(*s).is_some_and(|o| {
                    o.zone == Zone::Battlefield
                        && o.controller == player
                        && !o.status.contains(Status::PHASED_OUT)
                })
            })
    }

    /// Makes the waiting draws nobody can skip, in order, and stops at the
    /// first that `source` could replace: `(player, source)`. `None` once
    /// the queue is empty.
    pub(crate) fn next_draw_offer(&mut self) -> Option<(PlayerId, ObjectId)> {
        while let Some(&(player, n)) = self.draws_to_offer.front() {
            if n == 0 || self.has_left(player) {
                // Nobody draws for a player who has left (CR 800.4a:
                // what they own has left with them).
                self.draws_to_offer.pop_front();
                continue;
            }
            if let Some(source) = self.draw_skip_source(player, &[]) {
                return Some((player, source));
            }
            self.draws_to_offer.pop_front();
            self.draw_now(player, n as usize);
        }
        None
    }

    /// The answer about the draw at the front of the queue, asked of its
    /// player on behalf of `source`. Yes skips that one card's draw and
    /// makes the restriction. No leaves the draw to the next Sanctuary of
    /// theirs not yet asked about it, `declined` keeping who was: each
    /// replacement gets one opportunity at an event (CR 614.5); with none
    /// left the card is drawn. Then the queue goes on to its next question,
    /// if any.
    pub(crate) fn draw_offer_answered(
        &mut self,
        source: ObjectId,
        skip: bool,
        declined: &mut Vec<ObjectId>,
    ) -> Option<(PlayerId, ObjectId)> {
        let &(player, _) = self.draws_to_offer.front()?;
        if self.has_left(player) {
            // Asked, then gone before answering: nobody draws for them
            // (CR 800.4a: what they own has left with them), and their
            // question has no answer to settle.
            declined.clear();
            return self.next_draw_offer();
        }
        if skip {
            #[cfg(test)]
            crate::ability_log::replaced_by(source, ReplacementRule::MaySkipDrawStepDraw);
            self.restrict_attacks_after_skipped_draw(player, source);
            self.take_front_draw();
        } else {
            declined.push(source);
            if let Some(next) = self.draw_skip_source(player, declined) {
                return Some((player, next));
            }
            self.take_front_draw();
            self.draw_now(player, 1);
        }
        declined.clear();
        self.next_draw_offer()
    }

    /// One card of the front entry is done with.
    fn take_front_draw(&mut self) {
        if let Some(front) = self.draws_to_offer.front_mut() {
            front.1 = front.1.saturating_sub(1);
            if front.1 == 0 {
                self.draws_to_offer.pop_front();
            }
        }
    }

    /// The question about one draw: the Sanctuary's own optional
    /// replacement, keyed by its printed ability so a standing answer and a
    /// client label find it.
    pub(crate) fn draw_offer_question(&self, player: PlayerId, source: ObjectId) -> Pending {
        let ability = self.printed_ability_list(source).and_then(|list| {
            let index = list.abilities.iter().position(|a| {
                matches!(
                    a,
                    AbilityDef::Replacement(ReplacementRule::MaySkipDrawStepDraw)
                )
            })?;
            list.entry(index)?.provenance.ability_ref()
        });
        Pending::YesNo {
            player,
            prompt: YesNoPrompt::MayDo,
            source: ability,
        }
    }

    /// "If you do, until your next turn, you can't be attacked except by
    /// creatures with flying and/or islandwalk." Created by the skip and not
    /// by the permanent, so it holds once the source has left (the card's
    /// ruling); `Duration::UntilYourNextTurn` ends it as `player`'s next
    /// turn begins. A second skip before then changes nothing a first one
    /// did not, so it makes no second effect.
    pub(crate) fn restrict_attacks_after_skipped_draw(
        &mut self,
        player: PlayerId,
        source: ObjectId,
    ) {
        static FLYING_OR_ISLANDWALK: baylee_cards_dsl::Filter = baylee_cards_dsl::Filter::Or(&[
            baylee_cards_dsl::Filter::HasKeyword(baylee_cards_dsl::KeywordSet::FLYING),
            baylee_cards_dsl::Filter::HasKeyword(baylee_cards_dsl::KeywordSet::ISLANDWALK),
        ]);
        let modifier = baylee_cards_dsl::Modifier::CantBeAttackedExceptBy {
            who: baylee_cards_dsl::PlayerRel::You,
            by: &FLYING_OR_ISLANDWALK,
        };
        let duration = baylee_cards_dsl::Duration::UntilYourNextTurn;
        if self
            .effects
            .iter()
            .any(|fx| fx.controller == player && fx.modifier == modifier && fx.duration == duration)
        {
            return;
        }
        let timestamp = self.next_timestamp();
        self.effects.register(crate::effects::ContinuousEffect {
            // `register` assigns the real one.
            id: baylee_core::ids::EffectId::new(0),
            source: Some(source),
            controller: player,
            origin: crate::effects::EffectOrigin::Resolution,
            layer: modifier.layer(),
            timestamp,
            duration,
            filter: crate::effects::EffectFilter::Dsl(&baylee_cards_dsl::Filter::Any),
            modifier,
        });
    }
}
