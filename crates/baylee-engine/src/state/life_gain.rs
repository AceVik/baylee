//! Complete and amount-modifying replacements of a life gain, including the
//! affected player's ordering choice (CR 614.1a, CR 614.5, CR 616.1, CR 616.2).
//!
//! Lich turns a gain into draws; Sulfuric Vortex turns it into nothing; Boon
//! Reflection and Alhammarret's Archive multiply it; Cleric Class adds to it.
//! Each replacement applies to an event at most once (CR 614.5). When multiple
//! applicable replacements do not commute — such as `+1` beside `×2`, or an
//! amount modifier beside Lich's draws — the affected player chooses which
//! source applies first (CR 616.1, 616.1e), and remaining replacements
//! reconsider the modified event (CR 616.1f, 616.2). A choice waits in
//! `GameState::life_gains_to_offer`; later gains wait behind it, just as later
//! draws wait behind a draw choice.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary
use super::*;
use crate::choice::{ChoicePrompt, Pending};
use baylee_cards_dsl::{LifeGainOp, Modifier};
use baylee_core::ids::EffectId;

/// One positive life change waiting for its replacement order to be chosen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingLifeGain {
    /// The player who would gain life.
    pub player: PlayerId,
    /// Current amount of the gain after replacements applied so far.
    pub amount: i32,
    /// Why the original instruction caused the gain.
    pub cause: Cause,
    /// Continuous effects that have already modified this gain (CR 614.5).
    pub(crate) applied: smallvec::SmallVec<[EffectId; 2]>,
}

impl std::hash::Hash for PendingLifeGain {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.player.hash(state);
        self.amount.hash(state);
        self.cause.hash(state);
        if !self.applied.is_empty() {
            self.applied.hash(state);
        }
    }
}

/// One applicable effect and the public source a player can name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LifeGainReplacement {
    /// Identity of the continuous effect, so it applies at most once per gain.
    pub(crate) effect_id: EffectId,
    /// The object a player names to choose this effect.
    pub(crate) source: Option<ObjectId>,
    /// What the gain becomes or how its amount changes.
    pub(crate) kind: LifeGainReplacementKind,
}

/// Complete and arithmetic life-gain replacements the DSL can express.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LifeGainReplacementKind {
    /// Gain no life.
    NoLife,
    /// Draw that many cards.
    DrawCards,
    /// Multiply or add to the life gained (`Boon Reflection`, `Cleric Class`).
    Modify(LifeGainOp),
}

/// Whether every ordering of `replacements` produces the same outcome, or fewer
/// than two distinct public sources exist to choose between.
fn replacements_commute(replacements: &[LifeGainReplacement]) -> bool {
    let Some(first) = replacements.first() else {
        return true;
    };
    if replacements.len() == 1 {
        return true;
    }
    let mut sources: smallvec::SmallVec<[ObjectId; 4]> = smallvec::SmallVec::new();
    for r in replacements {
        if let Some(src) = r.source
            && !sources.contains(&src)
        {
            sources.push(src);
        }
    }
    if sources.len() < 2 {
        return true;
    }
    match first.kind {
        LifeGainReplacementKind::NoLife => replacements
            .iter()
            .all(|r| r.kind == LifeGainReplacementKind::NoLife),
        LifeGainReplacementKind::DrawCards => replacements
            .iter()
            .all(|r| r.kind == LifeGainReplacementKind::DrawCards),
        LifeGainReplacementKind::Modify(LifeGainOp::Multiply(_)) => replacements.iter().all(|r| {
            matches!(
                r.kind,
                LifeGainReplacementKind::Modify(LifeGainOp::Multiply(_))
            )
        }),
        LifeGainReplacementKind::Modify(LifeGainOp::Add(_)) => replacements
            .iter()
            .all(|r| matches!(r.kind, LifeGainReplacementKind::Modify(LifeGainOp::Add(_)))),
    }
}

impl GameState {
    /// Every currently applicable replacement of `player`'s gain that has not
    /// yet been applied to that event. Relations are read relative to each
    /// effect's own controller (or resolution-bound seats when created by a
    /// targeted effect).
    pub(crate) fn life_gain_replacements_for(
        &self,
        player: PlayerId,
        applied: &[EffectId],
    ) -> Vec<LifeGainReplacement> {
        if !self.effects.may_change_player_or_sba_rules() {
            return Vec::new();
        }
        self.effects
            .iter()
            .filter_map(|fx| {
                if applied.contains(&fx.id) {
                    return None;
                }
                let (kind, who) = match fx.modifier {
                    Modifier::LifeGainNoLifeInstead { who } => {
                        (LifeGainReplacementKind::NoLife, who)
                    }
                    Modifier::LifeGainDrawsInstead { who } => {
                        (LifeGainReplacementKind::DrawCards, who)
                    }
                    Modifier::ModifyLifeGain { who, op } => {
                        (LifeGainReplacementKind::Modify(op), who)
                    }
                    _ => return None,
                };
                let affects = if let Some(seats) = fx.filter.seats() {
                    !self.has_left(player) && seats.contains(player)
                } else {
                    crate::eval::players_for_source(who, self, fx.controller, fx.source)
                        .is_some_and(|players| players.contains(&player))
                };
                affects.then_some(LifeGainReplacement {
                    effect_id: fx.id,
                    source: fx.source,
                    kind,
                })
            })
            .collect()
    }

    /// Whether this gain becomes something other than a life gain (`NoLife`,
    /// `DrawCards`, or multiplication by zero). Simultaneous damage/floor
    /// calculations use this before a choice has been made: neither non-life
    /// replacement can enlarge the event's life-loss allowance, whereas a
    /// positive `ModifyLifeGain` remains a life gain.
    #[must_use]
    pub fn life_gain_is_replaced(&self, player: PlayerId) -> bool {
        self.life_gain_replacements_for(player, &[])
            .iter()
            .any(|r| {
                matches!(
                    r.kind,
                    LifeGainReplacementKind::NoLife
                        | LifeGainReplacementKind::DrawCards
                        | LifeGainReplacementKind::Modify(LifeGainOp::Multiply(0))
                )
            })
    }

    /// Applies an uncontested replacement chain, or queues a gain that must
    /// wait. Returns `true` when the ordinary unreplaced life change must not
    /// also happen.
    pub(crate) fn replace_or_queue_life_gain(
        &mut self,
        player: PlayerId,
        amount: i32,
        cause: Cause,
    ) -> bool {
        if self.has_left(player)
            || self.life_total_cant_change(player)
            || self.cant_gain_life(player)
        {
            return false;
        }
        if !self.life_gains_to_offer.is_empty() {
            self.life_gains_to_offer.push_back(PendingLifeGain {
                player,
                amount,
                cause,
                applied: smallvec::SmallVec::new(),
            });
            return true;
        }
        let replacements = self.life_gain_replacements_for(player, &[]);
        if replacements.is_empty() {
            return false;
        }
        if replacements_commute(&replacements) {
            self.settle_uncontested_life_gain(player, amount, cause, &replacements);
            return true;
        }
        self.life_gains_to_offer.push_back(PendingLifeGain {
            player,
            amount,
            cause,
            applied: smallvec::SmallVec::new(),
        });
        true
    }

    /// Advances to the first contested gain. Uncontested entries are settled
    /// here; a Lich replacement yields to its queued draws before a later gain.
    pub(crate) fn next_life_gain_offer(&mut self) -> Option<PlayerId> {
        if !self.draws_to_offer.is_empty() {
            return None;
        }
        while let Some(gain) = self.life_gains_to_offer.front().cloned() {
            if self.has_left(gain.player)
                || self.life_total_cant_change(gain.player)
                || self.cant_gain_life(gain.player)
                || gain.amount <= 0
            {
                self.life_gains_to_offer.pop_front();
                continue;
            }
            let replacements = self.life_gain_replacements_for(gain.player, &gain.applied);
            if !replacements_commute(&replacements) {
                return Some(gain.player);
            }
            self.life_gains_to_offer.pop_front();
            self.settle_uncontested_life_gain(gain.player, gain.amount, gain.cause, &replacements);
            if !self.draws_to_offer.is_empty() {
                return None;
            }
        }
        None
    }

    /// The distinct public sources of the currently applicable replacements for
    /// `player`'s front gain.
    fn life_gain_options(&self, player: PlayerId, applied: &[EffectId]) -> Vec<ObjectId> {
        let replacements = self.life_gain_replacements_for(player, applied);
        let mut options: Vec<ObjectId> = Vec::new();
        for replacement in replacements {
            if let Some(source) = replacement.source
                && !options.contains(&source)
            {
                options.push(source);
            }
        }
        options
    }

    /// Builds the affected player's source choice for the front gain.
    pub(crate) fn life_gain_question(&self, player: PlayerId) -> Pending {
        let applied = self
            .life_gains_to_offer
            .front()
            .filter(|gain| gain.player == player)
            .map_or(&[][..], |gain| gain.applied.as_slice());
        Pending::ChooseCards {
            player,
            options: self.life_gain_options(player, applied),
            min: 1,
            max: 1,
            prompt: ChoicePrompt::LifeGainReplacement,
            total: None,
        }
    }

    /// Applies the chosen replacement once, then advances to the next choice
    /// (settling any remaining uncontested replacements automatically).
    /// `Pending::answer_fault` has already restricted `chosen` to the menu.
    ///
    /// No answer names nothing: the asked player left the game, and that seat
    /// chooses nothing (CR 800.4f). The front gain is settled as if never
    /// asked about, which drops it if it was theirs (CR 800.4a) and applies
    /// whatever still stands if the sources that contested it have gone.
    pub(crate) fn life_gain_choice_answered(&mut self, chosen: &[ObjectId]) -> Option<PlayerId> {
        let gain = self.life_gains_to_offer.front().cloned()?;
        let named = chosen.first().copied();
        let chosen_replacement = self
            .life_gain_replacements_for(gain.player, &gain.applied)
            .into_iter()
            .find(|replacement| named.is_some() && replacement.source == named);
        if let Some(replacement) = chosen_replacement {
            self.note_life_gain_static_applied(replacement.effect_id);
            match replacement.kind {
                LifeGainReplacementKind::NoLife => {
                    self.life_gains_to_offer.pop_front();
                }
                LifeGainReplacementKind::DrawCards => {
                    self.life_gains_to_offer.pop_front();
                    self.draw_cards(gain.player, usize::try_from(gain.amount).unwrap_or(0));
                }
                LifeGainReplacementKind::Modify(op) => {
                    let next_amount = op.apply(gain.amount);
                    if next_amount <= 0 {
                        self.life_gains_to_offer.pop_front();
                    } else if let Some(front) = self.life_gains_to_offer.front_mut() {
                        front.amount = next_amount;
                        front.applied.push(replacement.effect_id);
                    }
                }
            }
        }
        self.next_life_gain_offer()
    }

    /// Applies a commuting sequence of remaining replacements and commits the
    /// resulting life change or draws.
    fn settle_uncontested_life_gain(
        &mut self,
        player: PlayerId,
        mut amount: i32,
        cause: Cause,
        replacements: &[LifeGainReplacement],
    ) {
        for replacement in replacements {
            self.note_life_gain_static_applied(replacement.effect_id);
            match replacement.kind {
                LifeGainReplacementKind::NoLife => return,
                LifeGainReplacementKind::DrawCards => {
                    self.draw_cards(player, usize::try_from(amount).unwrap_or(0));
                    return;
                }
                LifeGainReplacementKind::Modify(op) => {
                    amount = op.apply(amount);
                    if amount <= 0 {
                        return;
                    }
                }
            }
        }
        if amount > 0 {
            self.change_life_now(player, amount, cause);
        }
    }

    fn note_life_gain_static_applied(&self, effect_id: EffectId) {
        #[cfg(any(test, feature = "test-support"))]
        if let Some(fx) = self.effects.iter().find(|fx| fx.id == effect_id) {
            crate::ability_log::static_applied(fx);
        }
        #[cfg(not(any(test, feature = "test-support")))]
        let _ = (self, effect_id);
    }
}
