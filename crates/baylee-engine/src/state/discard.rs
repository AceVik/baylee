//! Discards an effect makes, and Library of Leng's way out of the graveyard.
//!
//! "To discard a card, move it from its owner's hand to that player's
//! graveyard" (CR 701.9a). Library of Leng replaces where the card goes and
//! nothing else: "discard it, but you may put it on top of your library
//! instead of into your graveyard". The answer is asked before the discard
//! is made (`resolve::discard`), kept in [`GameState::discard_answers`] and
//! spent here, the one door an effect's discard goes through.
//!
//! A card put into the library this way is not revealed, so its
//! characteristics are undefined to whatever reads the discard
//! (CR 701.9c), and the log names it only to who may see it.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary
use super::*;
use crate::object::Status;
use baylee_cards_dsl::ReplacementRule;

impl GameState {
    /// The Library of Leng `player` controls, if any: on the battlefield,
    /// not phased out, and still theirs.
    pub(crate) fn discard_top_source(&self, player: PlayerId) -> Option<ObjectId> {
        if self.has_left(player) {
            return None;
        }
        self.replacement_rules
            .iter()
            .filter(|r| r.rule == ReplacementRule::MayDiscardToLibraryTop && r.controller == player)
            .map(|r| r.source)
            .find(|s| {
                self.object(*s).is_some_and(|o| {
                    o.zone == Zone::Battlefield
                        && o.controller == player
                        && !o.status.contains(Status::PHASED_OUT)
                })
            })
    }

    /// Whether Library of Leng has been asked about `card` already.
    pub(crate) fn discard_answered(&self, card: ObjectId) -> bool {
        self.discard_answers.iter().any(|(c, _)| *c == card)
    }

    /// Writes down `player`'s arrangement of cards they are about to
    /// discard: `graveyard` as printed, `top` onto their library, listed
    /// top first.
    pub(crate) fn record_discard_answers(&mut self, graveyard: &[ObjectId], top: &[ObjectId]) {
        self.discards_on_top.clear();
        for &card in graveyard {
            self.discard_answers.push((card, None));
        }
        for (rank, &card) in top.iter().enumerate() {
            self.discard_answers
                .push((card, Some(u16::try_from(rank).unwrap_or(u16::MAX))));
        }
    }

    /// Drops answers no discard spent, once the instruction they were asked
    /// for is over.
    pub(crate) fn forget_discard_answers(&mut self) {
        self.discard_answers.clear();
        self.discards_on_top.clear();
    }

    /// `player` discards `card`, which is in their hand: the journal hears a
    /// discard (CR 701.9a) and the card goes to its owner's graveyard, or,
    /// where Library of Leng's answer says so, on top of their library.
    /// Only an effect's discard has an answer; a cost's or the cleanup
    /// step's never does.
    pub(crate) fn discard_card(&mut self, card: ObjectId, player: PlayerId, cause: Cause) {
        let owner = self.object(card).map_or(player, |o| o.owner);
        let answer = self
            .discard_answers
            .iter()
            .position(|(c, _)| *c == card)
            .map(|at| self.discard_answers.remove(at).1);
        self.journal.record(GameEvent::Discarded {
            object: card,
            player,
        });
        match answer {
            Some(Some(rank)) if cause == Cause::Effect => {
                let library = ZoneLocation::Library(owner);
                // Under every card of this answer already placed that was
                // ranked above it: the pile lies as it was listed.
                let above = self
                    .discards_on_top
                    .iter()
                    .filter(|(placed, r)| *r < rank && self.zones.list(library).contains(placed))
                    .count();
                let at = self.zones.list(library).len().saturating_sub(above);
                if self
                    .move_object(card, library, ZonePosition::Index(at), cause)
                    .is_ok()
                {
                    self.discards_on_top.push((card, rank));
                }
            }
            _ => {
                let _ = self.move_object(
                    card,
                    ZoneLocation::Graveyard(owner),
                    ZonePosition::Top,
                    cause,
                );
            }
        }
    }
}
