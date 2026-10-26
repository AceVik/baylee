//! Replacements of a card draw, offered card by card.
//!
//! A draw is an event of its own however many an instruction asks for: "If
//! a player is instructed to draw multiple cards, that player performs that
//! many individual card draws" (CR 121.2). Five things here can replace one:
//! Island Sanctuary's skip in its controller's draw step (CR 614.10), dredge
//! from the drawing player's graveyard (CR 702.52a), Chains of
//! Mephistopheles (every draw but the first of a draw step), and the next-draw
//! effects of Ring of Ma'rûf and Aladdin's Lamp. Each is applied even to a
//! player with no library (CR 121.6a, 614.11), and a replaced draw within a
//! sequence is finished before the sequence goes on (CR 614.11a).
//!
//! Sanctuary and dredge are optional and end the draw; Chains, Lamp and Ring
//! are not optional. The drawing player chooses which applies next when more
//! than one could and the order matters (CR 616.1, 616.1e): the optional ones
//! are offered first, and declining them is choosing a mandatory one or the
//! plain draw. Chains and Lamp leave a draw behind, a new opportunity for
//! every replacement not yet applied to it (CR 614.5, 616.1f, 616.2), so the
//! optional ones are asked again about that draw.
//!
//! `GameState::draw_cards` cannot ask, so a draw that could be replaced
//! waits in [`GameState::draws_to_offer`]. Two drainers ask about it: a
//! resolution, before its next instruction (`resolve::run`), and the
//! machine, for the turn-based draw and anything queued outside a
//! resolution (`Engine::offer_queued_draw`). Both build the same question
//! here and hand the answer back here; what the front draw has been through
//! so far is [`GameState::draw_front`].

#[allow(clippy::wildcard_imports)] // the parent's vocabulary
use super::*;
use crate::choice::{ChoicePrompt, Pending, YesNoPrompt};
use crate::object::Status;
use baylee_cards_dsl::{AbilityDef, Modifier, ReplacementRule};
use baylee_core::ids::EffectId;

/// The question about the waiting card draw at the front of the queue.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DrawReplacement {
    /// A permanent's optional draw-step skip (yes or no).
    Skip(ObjectId),
    /// Another player may pay life to put the revealed card in its owner's graveyard.
    Deny {
        /// Other player offered the payment in APNAP order.
        payer: PlayerId,
        /// The replacement's fixed life cost.
        life: u8,
    },
    /// One resolved Ring ability's next-draw replacement: which card from
    /// outside the game.
    Wish(EffectId),
    /// Which dredge card, if any, replaces the draw.
    Dredge,
    /// Which of several replacements applies next.
    Order,
    /// Which card this Chains of Mephistopheles has the player discard.
    Discard(ObjectId),
    /// Which of the cards an Aladdin's Lamp looked at go to the bottom: all
    /// but one.
    Bottom,
}

/// What the replacements have done so far to the draw at the front of
/// [`GameState::draws_to_offer`]; default between draws.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct FrontDraw {
    /// Island Sanctuaries whose skip the player declined for this draw.
    pub declined: Vec<ObjectId>,
    /// The player declined to dredge this draw.
    pub dredge_declined: bool,
    /// Chains of Mephistopheles already applied to this draw (CR 614.5).
    pub applied: Vec<ObjectId>,
    /// The cards an Aladdin's Lamp is looking at, top first.
    pub looked: Vec<ObjectId>,
    /// The original draw instruction was replaced (CR 614.11b).
    pub replaced: bool,
    /// Revealed exact library incarnation and remaining APNAP payment offers.
    pub denial: Option<(baylee_core::ids::DamageSourceRef, u8, Vec<PlayerId>)>,
}

/// A replacement that applies to the draw whether its player likes it or
/// not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mandatory {
    /// Chains of Mephistopheles.
    Chains(ObjectId),
    RevealMatch(ObjectId, &'static baylee_cards_dsl::Filter),
    RevealDeny(ObjectId, u8),
    /// Hullbreacher: replaces an opponent's draw (except the first in each of
    /// their draw steps) with its controller creating `token`.
    Hullbreacher {
        source: ObjectId,
        controller: PlayerId,
        token: &'static baylee_cards_dsl::TokenDef,
    },
    /// An Aladdin's Lamp effect.
    Lamp {
        effect: EffectId,
        source: Option<ObjectId>,
        count: u32,
    },
    /// A Ring of Ma'rûf effect.
    Wish {
        effect: EffectId,
        source: Option<ObjectId>,
    },
}

impl Mandatory {
    /// The object a player names to choose it.
    const fn source(self) -> Option<ObjectId> {
        match self {
            Self::Chains(source)
            | Self::RevealMatch(source, _)
            | Self::RevealDeny(source, _)
            | Self::Hullbreacher { source, .. } => Some(source),
            Self::Lamp { source, .. } | Self::Wish { source, .. } => source,
        }
    }
}

impl GameState {
    fn reveal_draw_replacements(&self, player: PlayerId) -> Vec<Mandatory> {
        self.replacement_rules
            .iter()
            .filter(|r| {
                !self.draw_front.applied.contains(&r.source)
                    && self.object(r.source).is_some_and(|o| {
                        o.zone == Zone::Battlefield && !o.status.contains(Status::PHASED_OUT)
                    })
            })
            .filter_map(|r| match r.rule {
                ReplacementRule::RevealDrawPutMatchingInGraveyard(filter)
                    if r.controller == player =>
                {
                    Some(Mandatory::RevealMatch(r.source, filter))
                }
                ReplacementRule::RevealDrawOthersMayPayLife(life) => {
                    Some(Mandatory::RevealDeny(r.source, life))
                }
                _ => None,
            })
            .collect()
    }

    fn reveal_draw_top(&mut self, player: PlayerId) -> Option<baylee_core::ids::DamageSourceRef> {
        let &card = self.zones.list(ZoneLocation::Library(player)).last()?;
        let version = self.object(card)?.version;
        self.journal.record(GameEvent::Revealed {
            player,
            cards: vec![card],
        });
        Some(baylee_core::ids::DamageSourceRef {
            object: card,
            version,
        })
    }

    fn deny_revealed_draw(&mut self, card: baylee_core::ids::DamageSourceRef) {
        let Some(owner) = self
            .source_object(card)
            .filter(|o| o.zone == Zone::Library)
            .map(|o| o.owner)
        else {
            return;
        };
        let _ = self.move_object(
            card.object,
            ZoneLocation::Graveyard(owner),
            ZonePosition::Top,
            Cause::Effect,
        );
    }

    fn next_draw_denial(&mut self) -> Option<DrawReplacement> {
        let (_, life, remaining) = self.draw_front.denial.as_ref()?;
        let life = *life;
        if let Some(&payer) = remaining
            .iter()
            .find(|&&p| !self.has_left(p) && self.can_pay_life(p, i32::from(life)))
        {
            return Some(DrawReplacement::Deny { payer, life });
        }
        self.draw_front.denial = None;
        self.reopen_front_draw();
        None
    }

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
            .filter(|r| {
                r.controller == player
                    && (r.rule == ReplacementRule::MaySkipDrawStepDraw
                        || (matches!(r.rule, ReplacementRule::MaySkipDrawStepGainLife(_))
                            && self.next_draw_is_first_in_draw_step(player)))
            })
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

    /// Whether some replacement could modify a draw `player` made now, so
    /// that the draw has to wait to be asked about.
    pub(crate) fn draw_may_be_replaced(&self, player: PlayerId) -> bool {
        self.draw_skip_source(player, &[]).is_some()
            || self.effects.iter().any(|effect| {
                effect.controller == player
                    && matches!(
                        effect.modifier,
                        Modifier::NextDrawWish | Modifier::NextDrawFromTop { .. }
                    )
            })
            || !self.reveal_draw_replacements(player).is_empty()
            || !self.chains_sources().is_empty()
            || !self.hullbreacher_sources(player).is_empty()
            || !self.dredge_options(player).is_empty()
    }

    /// Every Chains of Mephistopheles on the battlefield, phased in: it
    /// reads every player's draws, whoever controls it.
    fn chains_sources(&self) -> Vec<ObjectId> {
        let mut sources = Vec::new();
        for rule in &self.replacement_rules {
            if rule.rule == ReplacementRule::DiscardInsteadOfLaterDraws
                && !sources.contains(&rule.source)
                && self.object(rule.source).is_some_and(|o| {
                    o.zone == Zone::Battlefield && !o.status.contains(Status::PHASED_OUT)
                })
            {
                sources.push(rule.source);
            }
        }
        sources
    }

    /// Every Hullbreacher-style replacement on the battlefield controlled by
    /// an opponent of `player`.
    fn hullbreacher_sources(
        &self,
        player: PlayerId,
    ) -> Vec<(ObjectId, PlayerId, &'static baylee_cards_dsl::TokenDef)> {
        let mut sources = Vec::new();
        for rule in &self.replacement_rules {
            if let ReplacementRule::OpponentsExtraDrawsCreateToken(token) = rule.rule
                && self.is_opponent(player, rule.controller)
                && !sources.iter().any(|(s, _, _)| *s == rule.source)
                && self.object(rule.source).is_some_and(|o| {
                    o.zone == Zone::Battlefield && !o.status.contains(Status::PHASED_OUT)
                })
            {
                sources.push((rule.source, rule.controller, token));
            }
        }
        sources
    }

    /// The N of `card`'s dredge, read off its printed rules.
    fn dredge_count(&self, card: ObjectId) -> Option<u8> {
        self.printed_ability_list(card)?
            .abilities
            .iter()
            .find_map(|ability| match ability {
                AbilityDef::Replacement(ReplacementRule::Dredge(n)) => Some(*n),
                _ => None,
            })
    }

    /// The cards in `player`'s graveyard whose dredge could replace a draw
    /// of theirs now: cards they own (a token is no card, CR 108.2b) with
    /// at least as many cards in their library as the dredge mills
    /// (CR 702.52b).
    pub(crate) fn dredge_options(&self, player: PlayerId) -> Vec<ObjectId> {
        if self.has_left(player) {
            return Vec::new();
        }
        let library = self.zones.list(ZoneLocation::Library(player)).len();
        self.zones
            .list(ZoneLocation::Graveyard(player))
            .iter()
            .copied()
            .filter(|&card| {
                self.object(card)
                    .is_some_and(|o| o.owner == player && o.is_card())
                    && self
                        .dredge_count(card)
                        .is_some_and(|n| usize::from(n) <= library)
            })
            .collect()
    }

    /// The replacements that apply to `player`'s front draw whatever they
    /// choose, in the order they were made: Chains not yet applied to it
    /// (none to the first card of their draw step), then the next-draw
    /// effects of Aladdin's Lamp and Ring of Ma'rûf.
    fn mandatory_draw_replacements(&self, player: PlayerId) -> Vec<Mandatory> {
        let mut out = self.reveal_draw_replacements(player);
        if !self.next_draw_is_first_in_draw_step(player) {
            out.extend(
                self.chains_sources()
                    .into_iter()
                    .filter(|source| !self.draw_front.applied.contains(source))
                    .map(Mandatory::Chains),
            );
            out.extend(
                self.hullbreacher_sources(player)
                    .into_iter()
                    .filter(|(source, _, _)| !self.draw_front.applied.contains(source))
                    .map(|(source, controller, token)| Mandatory::Hullbreacher {
                        source,
                        controller,
                        token,
                    }),
            );
        }
        for effect in self.effects.iter().filter(|e| e.controller == player) {
            match effect.modifier {
                Modifier::NextDrawFromTop { count } => out.push(Mandatory::Lamp {
                    effect: effect.id,
                    source: effect.source,
                    count,
                }),
                Modifier::NextDrawWish => out.push(Mandatory::Wish {
                    effect: effect.id,
                    source: effect.source,
                }),
                _ => {}
            }
        }
        out
    }

    /// What the player may name when asked which replacement applies next:
    /// the dredge cards still on offer, then each mandatory one's source.
    fn draw_order_options(&self, player: PlayerId) -> Vec<ObjectId> {
        let mut options = if self.draw_front.dredge_declined {
            Vec::new()
        } else {
            self.dredge_options(player)
        };
        for source in self
            .mandatory_draw_replacements(player)
            .into_iter()
            .filter_map(Mandatory::source)
        {
            if !options.contains(&source) {
                options.push(source);
            }
        }
        options
    }

    /// Makes the waiting draws nothing can replace, in order, applies the
    /// replacements nobody has to choose about, and stops at the first
    /// question: `(player, question)`. `None` once the queue is empty.
    pub(crate) fn next_draw_offer(&mut self) -> Option<(PlayerId, DrawReplacement)> {
        while let Some(&(player, n)) = self.draws_to_offer.front() {
            if n == 0
                || self.has_left(player)
                || self
                    .draw_limit(player)
                    .is_some_and(|limit| self.per_turn.draws[usize::from(player.get())] >= limit)
            {
                // Nobody draws for a player who has left (CR 800.4a:
                // what they own has left with them).
                self.draws_to_offer.pop_front();
                self.draw_front = FrontDraw::default();
                continue;
            }
            if let Some(question) = self.front_draw_step(player, n) {
                return Some((player, question));
            }
        }
        None
    }

    /// One step for the front draw: the question to ask about it, or `None`
    /// once something happened to it without one.
    fn front_draw_step(&mut self, player: PlayerId, n: u32) -> Option<DrawReplacement> {
        if let Some(offer) = self.next_draw_denial() {
            return Some(offer);
        }
        if let Some(source) = self.draw_skip_source(player, &self.draw_front.declined) {
            return Some(DrawReplacement::Skip(source));
        }
        let dredging = !self.draw_front.dredge_declined && !self.dredge_options(player).is_empty();
        let mandatory = self.mandatory_draw_replacements(player);
        if dredging {
            return Some(if mandatory.is_empty() {
                DrawReplacement::Dredge
            } else {
                DrawReplacement::Order
            });
        }
        let Some(&first) = mandatory.first() else {
            self.draw_plainly(player, n);
            return None;
        };
        // Replacements that do the same thing are the same in either order
        // (two Chains of one controller, two Rings, two Lamps of one X);
        // anything else in company is the player's choice (CR 616.1e). Two
        // players' Chains are not the same: the discard is the Chains'
        // controller's effect (Psychic Purge reads whose).
        let controller = |m: &Mandatory| {
            m.source()
                .and_then(|s| self.object(s))
                .map(|o| o.controller)
        };
        let interchangeable = (mandatory.iter().all(|m| matches!(m, Mandatory::Chains(_)))
            && mandatory
                .windows(2)
                .all(|pair| controller(&pair[0]) == controller(&pair[1])))
            || mandatory
                .iter()
                .all(|m| matches!(m, Mandatory::Wish { .. }))
            || mandatory.windows(2).all(|pair| {
                matches!(
                    pair,
                    [Mandatory::Lamp { count: a, .. }, Mandatory::Lamp { count: b, .. }] if a == b
                )
            });
        if !interchangeable && self.draw_order_options(player).len() > 1 {
            return Some(DrawReplacement::Order);
        }
        self.apply_mandatory_draw_replacement(player, first)
    }

    /// The front card is drawn as it is. A whole multiple draw at once
    /// unless a replacement has begun on this card or Chains could treat
    /// the cards of it differently (the first of a draw step is exempt).
    fn draw_plainly(&mut self, player: PlayerId, n: u32) {
        if self.draw_front != FrontDraw::default()
            || !self.chains_sources().is_empty()
            || !self.hullbreacher_sources(player).is_empty()
        {
            let replaced = self.draw_front.replaced;
            let before = self.per_turn.instruction_drawn.len();
            self.take_front_draw();
            self.draw_now(player, 1);
            if replaced {
                self.per_turn.instruction_drawn.truncate(before);
            }
        } else {
            self.draws_to_offer.pop_front();
            self.draw_now(player, n as usize);
        }
    }

    /// Applies `replacement` to `player`'s front draw: the question it asks,
    /// or `None` once it has done its part.
    fn apply_mandatory_draw_replacement(
        &mut self,
        player: PlayerId,
        replacement: Mandatory,
    ) -> Option<DrawReplacement> {
        self.draws_replaced = self.draws_replaced.wrapping_add(1);
        self.draw_front.replaced = true;
        match replacement {
            Mandatory::RevealMatch(source, filter) => {
                #[cfg(test)]
                crate::ability_log::replaced_by(
                    source,
                    ReplacementRule::RevealDrawPutMatchingInGraveyard(filter),
                );
                self.draw_front.applied.push(source);
                if let Some(card) = self.reveal_draw_top(player) {
                    let matches = self
                        .object(card.object)
                        .is_some_and(|o| crate::eval::matches(filter, self, o, player, source));
                    if matches {
                        self.deny_revealed_draw(card);
                        self.take_front_draw();
                    } else {
                        self.reopen_front_draw();
                    }
                } else {
                    self.reopen_front_draw();
                }
                None
            }
            Mandatory::RevealDeny(source, life) => self.apply_draw_denial(player, source, life),
            Mandatory::Chains(source) => {
                #[cfg(test)]
                crate::ability_log::replaced_by(
                    source,
                    ReplacementRule::DiscardInsteadOfLaterDraws,
                );
                self.draw_front.applied.push(source);
                let hand = self.zones.list(ZoneLocation::Hand(player)).clone();
                match hand.as_slice() {
                    [] => {
                        // "If the player doesn't discard a card this way,
                        // they mill a card", and no card is drawn at all
                        // (the card's ruling).
                        self.mill_top(player, 1);
                        self.take_front_draw();
                        None
                    }
                    [only] => {
                        self.chains_discard(player, source, *only);
                        None
                    }
                    _ => Some(DrawReplacement::Discard(source)),
                }
            }
            Mandatory::Hullbreacher {
                source,
                controller,
                token,
            } => {
                let _ = source;
                #[cfg(test)]
                crate::ability_log::replaced_by(
                    source,
                    ReplacementRule::OpponentsExtraDrawsCreateToken(token),
                );
                let _ = crate::resolve::tokens::create_tokens(self, controller, token, None, 1);
                self.take_front_draw();
                None
            }
            Mandatory::Lamp { effect, count, .. } => {
                self.effects.remove_where(|entry| entry.id == effect);
                let looked: Vec<ObjectId> = self
                    .zones
                    .list(ZoneLocation::Library(player))
                    .iter()
                    .rev()
                    .take(count as usize)
                    .copied()
                    .collect();
                if looked.len() > 1 {
                    self.draw_front.looked = looked;
                    return Some(DrawReplacement::Bottom);
                }
                // One card or none: nothing goes to the bottom, and the
                // draw it ends with is made (or replaced) next.
                self.reopen_front_draw();
                None
            }
            Mandatory::Wish { effect, .. } => {
                if self.draw_wish_options(player).is_empty() {
                    // Nothing to put into the hand: the draw is replaced
                    // by nothing (CR 614.6), and the effect is spent.
                    self.effects.remove_where(|entry| entry.id == effect);
                    self.take_front_draw();
                    return None;
                }
                Some(DrawReplacement::Wish(effect))
            }
        }
    }

    fn apply_draw_denial(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        life: u8,
    ) -> Option<DrawReplacement> {
        #[cfg(test)]
        crate::ability_log::replaced_by(source, ReplacementRule::RevealDrawOthersMayPayLife(life));
        self.draw_front.applied.push(source);
        if let Some(card) = self.reveal_draw_top(player) {
            let seats = self.players.len();
            let active = usize::from(self.turn.active.get());
            let payers = (0..seats)
                .map(|offset| {
                    PlayerId::new(u8::try_from((active + offset) % seats).expect("seat fits"))
                })
                .filter(|&seat| {
                    seat != player
                        && !self.has_left(seat)
                        && self.can_pay_life(seat, i32::from(life))
                })
                .collect();
            self.draw_front.denial = Some((card, life, payers));
            self.next_draw_denial()
        } else {
            self.reopen_front_draw();
            None
        }
    }

    /// The draw a replacement left behind is a new event to the optional
    /// replacements (CR 616.1f, 616.2): asked again, Chains already applied
    /// excepted.
    fn reopen_front_draw(&mut self) {
        self.draw_front.declined.clear();
        self.draw_front.dredge_declined = false;
        self.draw_front.looked.clear();
    }

    /// Chains of Mephistopheles' discard: an effect's (CR 701.9a), the
    /// Chains' controller's; then the draw it leaves.
    fn chains_discard(&mut self, player: PlayerId, source: ObjectId, card: ObjectId) {
        let by = self.object(source).map(|o| o.controller);
        self.discard_card(card, player, crate::event::Cause::Effect, by);
        self.reopen_front_draw();
    }

    /// Dredge: `card` returns from `player`'s graveyard to their hand and
    /// they mill its count instead of drawing (CR 702.52a).
    fn dredge(&mut self, player: PlayerId, card: ObjectId) {
        let Some(n) = self.dredge_count(card) else {
            return;
        };
        #[cfg(test)]
        crate::ability_log::replaced_by(card, ReplacementRule::Dredge(n));
        self.draws_replaced = self.draws_replaced.wrapping_add(1);
        self.mill_top(player, usize::from(n));
        let _ = self.move_object(
            card,
            ZoneLocation::Hand(player),
            ZonePosition::Top,
            crate::event::Cause::Effect,
        );
        self.take_front_draw();
    }

    /// The yes-or-no answer about the front draw, asked of its player on
    /// behalf of an Island Sanctuary. Yes skips that one card's draw and
    /// makes the restriction. No leaves the draw to whatever else may
    /// replace it: each replacement gets one opportunity at an event
    /// (CR 614.5). Then the queue goes on to its next question, if any.
    pub(crate) fn draw_offer_answered(
        &mut self,
        replacement: DrawReplacement,
        skip: bool,
    ) -> Option<(PlayerId, DrawReplacement)> {
        if let DrawReplacement::Deny { payer, life } = replacement {
            if skip && self.can_pay_life(payer, i32::from(life)) {
                if let Some((card, _, _)) = self.draw_front.denial.take() {
                    self.change_life(payer, -i32::from(life), crate::event::Cause::Cost);
                    self.deny_revealed_draw(card);
                    self.take_front_draw();
                }
            } else if let Some((_, _, remaining)) = &mut self.draw_front.denial
                && let Some(index) = remaining.iter().position(|seat| *seat == payer)
            {
                remaining.remove(index);
            }
            return self.next_draw_offer();
        }
        let DrawReplacement::Skip(source) = replacement else {
            return self.next_draw_offer();
        };
        let &(player, _) = self.draws_to_offer.front()?;
        if self.has_left(player) {
            // Asked, then gone before answering: nobody draws for them
            // (CR 800.4a: what they own has left with them), and their
            // question has no answer to settle.
            self.draw_front = FrontDraw::default();
            return self.next_draw_offer();
        }
        if skip {
            let gain = self.replacement_rules.iter().find_map(|r| {
                if r.source == source
                    && r.controller == player
                    && let ReplacementRule::MaySkipDrawStepGainLife(n) = r.rule
                {
                    return Some(n);
                }
                None
            });
            if let Some(life) = gain {
                #[cfg(test)]
                crate::ability_log::replaced_by(
                    source,
                    ReplacementRule::MaySkipDrawStepGainLife(life),
                );
                self.draws_replaced = self.draws_replaced.wrapping_add(1);
                self.change_life(player, i32::from(life), crate::event::Cause::Effect);
                self.take_front_draw();
            } else {
                #[cfg(test)]
                crate::ability_log::replaced_by(source, ReplacementRule::MaySkipDrawStepDraw);
                self.draws_replaced = self.draws_replaced.wrapping_add(1);
                self.restrict_attacks_after_skipped_draw(player, source);
                self.take_front_draw();
            }
        } else {
            self.draw_front.declined.push(source);
        }
        self.next_draw_offer()
    }

    /// The cards named in answer to a question about the front draw, then
    /// the next question, if any.
    pub(crate) fn draw_choice_answered(
        &mut self,
        replacement: DrawReplacement,
        chosen: &[ObjectId],
    ) -> Option<(PlayerId, DrawReplacement)> {
        let &(player, _) = self.draws_to_offer.front()?;
        if self.has_left(player) {
            self.draw_front = FrontDraw::default();
            return self.next_draw_offer();
        }
        match replacement {
            DrawReplacement::Skip(_) | DrawReplacement::Deny { .. } => {}
            DrawReplacement::Wish(effect) => {
                self.draw_wish_answered(player, effect, chosen.first().copied());
            }
            DrawReplacement::Dredge => match chosen.first() {
                Some(&card) if self.dredge_options(player).contains(&card) => {
                    self.dredge(player, card);
                }
                _ => self.draw_front.dredge_declined = true,
            },
            DrawReplacement::Order => {
                let named = chosen.first().copied();
                if let Some(card) = named.filter(|card| {
                    !self.draw_front.dredge_declined && self.dredge_options(player).contains(card)
                }) {
                    self.dredge(player, card);
                } else {
                    let mandatory = self.mandatory_draw_replacements(player);
                    let pick = mandatory
                        .iter()
                        .copied()
                        .find(|m| named.is_some() && m.source() == named)
                        .or_else(|| mandatory.first().copied());
                    if let Some(pick) = pick
                        && let Some(next) = self.apply_mandatory_draw_replacement(player, pick)
                    {
                        return Some((player, next));
                    }
                }
            }
            DrawReplacement::Discard(source) => {
                let hand = self.zones.list(ZoneLocation::Hand(player));
                let card = chosen
                    .first()
                    .copied()
                    .filter(|card| hand.contains(card))
                    .or_else(|| hand.first().copied());
                if let Some(card) = card {
                    self.chains_discard(player, source, card);
                }
            }
            DrawReplacement::Bottom => {
                let looked = std::mem::take(&mut self.draw_front.looked);
                let mut bottom: Vec<ObjectId> = chosen
                    .iter()
                    .copied()
                    .filter(|card| looked.contains(card))
                    .collect();
                bottom.dedup();
                if bottom.len() + 1 != looked.len() {
                    // Never reached through `apply`, which holds the
                    // answer to the question; the top card stays.
                    bottom = looked.iter().skip(1).copied().collect();
                }
                self.put_on_bottom_in_random_order(player, bottom);
                self.reopen_front_draw();
            }
        }
        self.next_draw_offer()
    }

    /// One card of the front entry is done with, and so is what the
    /// replacements did to it.
    pub(super) fn take_front_draw(&mut self) {
        if let Some(front) = self.draws_to_offer.front_mut() {
            front.1 = front.1.saturating_sub(1);
            if front.1 == 0 {
                self.draws_to_offer.pop_front();
            }
        }
        self.draw_front = FrontDraw::default();
    }

    /// The question about one draw.
    pub(crate) fn draw_offer_question(
        &self,
        player: PlayerId,
        replacement: DrawReplacement,
    ) -> Pending {
        let choose =
            |options: Vec<ObjectId>, min: u8, max: u8, prompt: ChoicePrompt| Pending::ChooseCards {
                player,
                options,
                min,
                max,
                prompt,
                total: None,
            };
        match replacement {
            DrawReplacement::Deny { payer, life } => Pending::YesNo {
                player: payer,
                prompt: YesNoPrompt::PayLife {
                    amount: u32::from(life),
                },
                source: None,
            },
            DrawReplacement::Skip(source) => self.draw_skip_question(player, source),
            DrawReplacement::Wish(_) => self.draw_wish_question(player),
            DrawReplacement::Dredge => {
                choose(self.dredge_options(player), 0, 1, ChoicePrompt::Dredge)
            }
            DrawReplacement::Order => choose(
                self.draw_order_options(player),
                1,
                1,
                ChoicePrompt::DrawReplacement,
            ),
            DrawReplacement::Discard(_) => choose(
                self.zones.list(ZoneLocation::Hand(player)).clone(),
                1,
                1,
                ChoicePrompt::Discard,
            ),
            DrawReplacement::Bottom => {
                let looked = self.draw_front.looked.clone();
                let n = u8::try_from(looked.len().saturating_sub(1)).unwrap_or(u8::MAX);
                choose(looked, n, n, ChoicePrompt::PutOnBottom)
            }
        }
    }

    /// The Sanctuary's own optional replacement, keyed by its printed
    /// ability so a standing answer and a client label find it.
    fn draw_skip_question(&self, player: PlayerId, source: ObjectId) -> Pending {
        let ability = self.printed_ability_list(source).and_then(|list| {
            let index = list.abilities.iter().position(|a| {
                matches!(
                    a,
                    AbilityDef::Replacement(
                        ReplacementRule::MaySkipDrawStepDraw
                            | ReplacementRule::MaySkipDrawStepGainLife(_)
                    )
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
