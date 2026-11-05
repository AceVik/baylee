//! Complete replacements of a life gain, including the affected player's choice.
//!
//! Lich turns a gain into draws; Sulfuric Vortex turns it into nothing. Both
//! replace the entire event, so after either there is no gain left for the
//! other (CR 614.5, 616.1f). When both apply, the affected player chooses one
//! (CR 616.1, 616.1e). A choice waits in `GameState::life_gains_to_offer`;
//! later gains wait behind it, just as later draws wait behind a draw choice.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary
use super::*;
use crate::choice::{ChoicePrompt, Pending};
use baylee_cards_dsl::Modifier;

/// One positive life change waiting for its replacement to be chosen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PendingLifeGain {
    /// The player who would gain life.
    pub player: PlayerId,
    /// The amount of the original gain.
    pub amount: i32,
    /// Why the original instruction caused the gain.
    pub cause: Cause,
}

/// One applicable effect and the public source a player can name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LifeGainReplacement {
    /// The object a player names to choose this effect.
    pub(crate) source: Option<ObjectId>,
    /// What the gain becomes.
    pub(crate) kind: LifeGainReplacementKind,
}

/// The two complete replacements the DSL can currently print.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LifeGainReplacementKind {
    /// Gain no life.
    NoLife,
    /// Draw that many cards.
    DrawCards,
}

impl GameState {
    /// Every currently applicable complete replacement of `player`'s gain.
    /// Relations are read relative to each effect's own controller.
    pub(crate) fn life_gain_replacements(&self, player: PlayerId) -> Vec<LifeGainReplacement> {
        if !self.effects.may_change_player_or_sba_rules() {
            return Vec::new();
        }
        self.effects
            .iter()
            .filter_map(|fx| {
                let (kind, who) = match fx.modifier {
                    Modifier::LifeGainNoLifeInstead { who } => {
                        (LifeGainReplacementKind::NoLife, who)
                    }
                    Modifier::LifeGainDrawsInstead { who } => {
                        (LifeGainReplacementKind::DrawCards, who)
                    }
                    _ => return None,
                };
                crate::eval::players(who, self, fx.controller)
                    .is_some_and(|players| players.contains(&player))
                    .then_some(LifeGainReplacement {
                        source: fx.source,
                        kind,
                    })
            })
            .collect()
    }

    /// Whether this gain becomes something other than a life gain. Simultaneous
    /// damage/floor calculations use this before a choice has been made: neither
    /// available replacement can enlarge the event's life-loss allowance.
    #[must_use]
    pub fn life_gain_is_replaced(&self, player: PlayerId) -> bool {
        !self.life_gain_replacements(player).is_empty()
    }

    /// Applies an uncontested replacement, or queues a gain that must wait.
    /// Returns `true` when the ordinary life change must not also happen.
    pub(crate) fn replace_or_queue_life_gain(
        &mut self,
        player: PlayerId,
        amount: i32,
        cause: Cause,
    ) -> bool {
        if self.has_left(player) || self.life_total_cant_change(player) {
            return false;
        }
        if !self.life_gains_to_offer.is_empty() {
            self.life_gains_to_offer.push_back(PendingLifeGain {
                player,
                amount,
                cause,
            });
            return true;
        }
        let replacements = self.life_gain_replacements(player);
        let Some(first) = replacements.first() else {
            return false;
        };
        if replacements
            .iter()
            .all(|replacement| replacement.kind == first.kind)
        {
            self.apply_life_gain_replacement(player, amount, first.kind);
            return true;
        }
        self.life_gains_to_offer.push_back(PendingLifeGain {
            player,
            amount,
            cause,
        });
        true
    }

    /// Advances to the first contested gain. Uncontested entries are settled
    /// here; a Lich replacement yields to its queued draws before a later gain.
    pub(crate) fn next_life_gain_offer(&mut self) -> Option<PlayerId> {
        if !self.draws_to_offer.is_empty() {
            return None;
        }
        while let Some(gain) = self.life_gains_to_offer.front().copied() {
            if self.has_left(gain.player) || self.life_total_cant_change(gain.player) {
                self.life_gains_to_offer.pop_front();
                continue;
            }
            let replacements = self.life_gain_replacements(gain.player);
            let contested = replacements.first().is_some_and(|first| {
                replacements
                    .iter()
                    .any(|replacement| replacement.kind != first.kind)
            });
            if contested {
                return Some(gain.player);
            }
            self.life_gains_to_offer.pop_front();
            if let Some(first) = replacements.first() {
                self.apply_life_gain_replacement(gain.player, gain.amount, first.kind);
            } else {
                self.change_life_now(gain.player, gain.amount, gain.cause);
            }
            if !self.draws_to_offer.is_empty() {
                return None;
            }
        }
        None
    }

    /// The sources of the currently applicable, meaningfully different choices.
    ///
    /// This shared object-choice protocol names sources, not `EffectId`s. The
    /// card lint therefore forbids different kinds on one source; check the
    /// invariant here too rather than silently picking one in a hand-built game.
    fn life_gain_options(&self, player: PlayerId) -> Vec<ObjectId> {
        let replacements = self.life_gain_replacements(player);
        let mut options: Vec<(ObjectId, LifeGainReplacementKind)> = Vec::new();
        for replacement in replacements {
            let source = replacement
                .source
                .expect("a contested life-gain replacement needs a public source");
            if let Some((_, earlier)) = options.iter().find(|(id, _)| *id == source) {
                assert_eq!(
                    *earlier, replacement.kind,
                    "one source cannot hide two different life-gain replacements"
                );
            } else {
                options.push((source, replacement.kind));
            }
        }
        options.into_iter().map(|(source, _)| source).collect()
    }

    /// Builds the affected player's source choice for the front gain.
    pub(crate) fn life_gain_question(&self, player: PlayerId) -> Pending {
        Pending::ChooseCards {
            player,
            options: self.life_gain_options(player),
            min: 1,
            max: 1,
            prompt: ChoicePrompt::LifeGainReplacement,
            total: None,
        }
    }

    /// Applies the chosen replacement once, then advances to the next choice.
    /// `Pending::answer_fault` has already restricted `chosen` to the menu.
    pub(crate) fn life_gain_choice_answered(&mut self, chosen: &[ObjectId]) -> Option<PlayerId> {
        let gain = self.life_gains_to_offer.front().copied()?;
        let replacements = self.life_gain_replacements(gain.player);
        let named = chosen.first().copied();
        let kind = replacements
            .iter()
            .find(|replacement| named.is_some() && replacement.source == named)
            .expect("the offered source identifies an applicable replacement")
            .kind;
        self.life_gains_to_offer.pop_front();
        self.apply_life_gain_replacement(gain.player, gain.amount, kind);
        self.next_life_gain_offer()
    }

    /// A complete replacement leaves no life-gain event, tally or journal entry.
    fn apply_life_gain_replacement(
        &mut self,
        player: PlayerId,
        amount: i32,
        kind: LifeGainReplacementKind,
    ) {
        if kind == LifeGainReplacementKind::DrawCards {
            self.draw_cards(player, usize::try_from(amount).unwrap_or(0));
        }
    }
}
