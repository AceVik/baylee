//! The choices a controlled player is asked as their spells and abilities
//! are cast and resolve (CR 724.5), made against them.
//!
//! Each reading measures from their side of the table (`theirs`, the view
//! [`super::seen_by`] builds) and takes the other end of what they would
//! take: their best cards are the ones discarded, sacrificed and put back,
//! their worst the ones kept and found; a target, a mode and an X are the
//! worst for them by the same [`crate::worth`] reading that chose to cast
//! the spell; a "may" is done only when it costs them. A question this file
//! has no reading for is `None`, and the ordinary answer takes it.

use baylee_cards_dsl::Effect;
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_engine::choice::{
    ArrangePrompt, CastModeDesc, CastModeKind, ChoicePrompt, NumberPrompt, Pending, PlayerAction,
    TargetPrompt, YesNoPrompt,
};
use baylee_engine::engine::DecisionContext;
use baylee_view::PlayerView;

use crate::HeuristicAgent;
use crate::worth::{Aim, Origin, THRESHOLD};

/// Whether what an "unless" does when its price goes unpaid (CR 118.12a)
/// counters a spell or an ability: ward, Mana Leak, Power Sink.
pub(crate) fn counters(effect: &Effect) -> bool {
    match effect {
        Effect::Sequence(effects) => effects.iter().any(counters),
        Effect::CounterTargetSpell
        | Effect::CounterTargetSpellToExile
        | Effect::CounterTargetAbility
        | Effect::CounterTargetSpellOrAbility => true,
        _ => false,
    }
}

/// Whether to pay a tax for them: only when refusing it counters their
/// spell, which this seat cast because it hurts them, and the mana is there.
fn taxed_against(
    view: &PlayerView,
    cost: baylee_core::mana::ManaCost,
    context: &DecisionContext<'_>,
) -> bool {
    context.effects.iter().any(|effect| match effect {
        Effect::PlayerMayPayOr { effect, .. } | Effect::PlayerMayPayManaOr { effect, .. } => {
            counters(effect)
        }
        _ => false,
    }) && crate::policy::can_pay(view, &cost)
}

/// `options` ranked by `value`, most valuable first, ties by handle.
fn ranked(options: &[ObjectId], value: impl Fn(&ObjectId) -> i64) -> Vec<ObjectId> {
    let mut ranked = options.to_vec();
    ranked.sort_by_cached_key(|id| (std::cmp::Reverse(value(id)), *id));
    ranked
}

/// A stable order for target ties: objects before players, each by handle.
fn aim_key(aim: Aim) -> (u8, u64) {
    match aim {
        Aim::Object(id) => (0, u64::from(id.slot())),
        Aim::Player(p) => (1, u64::from(p.get())),
        Aim::Source | Aim::Nothing => (2, 0),
    }
}

impl HeuristicAgent {
    /// Every question but priority and the attack (`super`).
    #[allow(clippy::too_many_lines)] // the pending taxonomy is one flat table
    pub(super) fn choice_against(
        &self,
        view: &PlayerView,
        theirs: &PlayerView,
        pending: &Pending,
        context: &DecisionContext<'_>,
    ) -> Option<PlayerAction> {
        match pending {
            // Their best cards, by what each is worth to them now.
            Pending::DiscardChoice { count, .. } | Pending::MulliganBottom { count, .. } => {
                let hand: Vec<ObjectId> = theirs.hand.iter().map(|card| card.id).collect();
                let mut objects = ranked(&hand, Self::card_value(theirs));
                objects.truncate(usize::from(*count));
                Some(PlayerAction::ChooseObjects { objects })
            }
            // The legend rule keeps one (CR 704.5j): the one worth least.
            Pending::LegendChoice { options, .. } => {
                let kept = ranked(options, |id| self.their_worth(theirs, *id))
                    .pop()
                    .or_else(|| options.first().copied())?;
                Some(PlayerAction::ChooseObjects {
                    objects: vec![kept],
                })
            }
            Pending::ChooseCards {
                options,
                min,
                max,
                prompt,
                total,
                ..
            } => self
                .cards_against(
                    theirs,
                    options,
                    *min,
                    *max,
                    *prompt,
                    total.is_some(),
                    context,
                )
                .map(|objects| PlayerAction::ChooseObjects { objects }),
            // A cost being paid, and a target being changed (a redirect's
            // own reading, `redirect`), are not choices of a fresh target.
            Pending::ChooseTargets {
                reason:
                    TargetPrompt::Convoke
                    | TargetPrompt::DistinctManaValues(_)
                    | TargetPrompt::Retarget { .. },
                ..
            } => None,
            Pending::ChooseTargets {
                options,
                player_options,
                min,
                max,
                ..
            } => self.targets_against(theirs, options, player_options, *min, *max, context),
            Pending::ChooseCastMode {
                object, options, ..
            } => self
                .mode_against(theirs, *object, options)
                .map(PlayerAction::ChooseMode),
            Pending::ChooseNumber {
                min, max, reason, ..
            } => self
                .number_against(view, theirs, *min, *max, *reason, context)
                .map(PlayerAction::ChooseNumber),
            // Fact or Fiction's pile into their hand: the one worth least.
            Pending::ChoosePile {
                piles, label: None, ..
            } => {
                let value = Self::card_value(theirs);
                let worth =
                    |pile: &Vec<ObjectId>| pile.iter().map(|id| value(id).max(0)).sum::<i64>();
                (0..piles.len())
                    .min_by_key(|&i| (worth(&piles[i]), piles[i].len(), i))
                    .map(PlayerAction::ChooseMode)
            }
            // A scry or a surveil keeps on top what is worth least to them
            // and sends away what is worth most.
            Pending::Arrange {
                cards,
                piles,
                prompt: ArrangePrompt::Scry | ArrangePrompt::Surveil,
                ..
            } if piles.len() == 2 => {
                let value = Self::card_value(theirs);
                let mut away: Vec<ObjectId> =
                    cards.iter().copied().filter(|id| value(id) > 0).collect();
                away.sort_by_key(|id| (std::cmp::Reverse(value(id)), *id));
                let top = cards
                    .iter()
                    .copied()
                    .filter(|id| !away.contains(id))
                    .collect();
                Some(PlayerAction::Arrange {
                    piles: vec![top, away],
                })
            }
            // Library of Leng: what they are losing goes to the graveyard
            // and not back on top as their next draw.
            Pending::Arrange {
                cards,
                piles,
                prompt: ArrangePrompt::DiscardToLibrary,
                ..
            } if piles.len() == 2 => Some(PlayerAction::Arrange {
                piles: vec![cards.clone(), Vec::new()],
            }),
            Pending::YesNo { prompt, source, .. } => self
                .yes_no_against(view, theirs, *prompt, *source, context)
                .map(PlayerAction::YesNo),
            _ => None,
        }
    }

    /// What `id` is worth to the player `theirs` belongs to: a permanent by
    /// what giving it up costs them, a card by its value in their hand.
    fn their_worth(&self, theirs: &PlayerView, id: ObjectId) -> i64 {
        theirs.battlefield.iter().find(|o| o.id == id).map_or_else(
            || Self::card_value(theirs)(&id),
            |o| self.given_up(theirs, o),
        )
    }

    /// The cards to name from `options`, or `None` for a question this file
    /// has no reading for.
    #[allow(clippy::too_many_arguments)] // the question's own fields
    fn cards_against(
        &self,
        theirs: &PlayerView,
        options: &[ObjectId],
        min: u8,
        max: u8,
        prompt: ChoicePrompt,
        totalled: bool,
        context: &DecisionContext<'_>,
    ) -> Option<Vec<ObjectId>> {
        // A total (crew, saddle) is a price for something already chosen.
        if totalled {
            return None;
        }
        let best = |n: u8| {
            let mut chosen = ranked(options, |id| self.their_worth(theirs, *id));
            chosen.truncate(usize::from(n));
            chosen
        };
        let worst = |n: u8| {
            let mut chosen = ranked(options, |id| self.their_worth(theirs, *id));
            chosen.reverse();
            chosen.truncate(usize::from(n));
            chosen
        };
        Some(match prompt {
            // A price, or a loss: paid with the most they have to lose, and
            // no more of it than the question makes them. An "unless" asks
            // with `min` 0, and naming nothing is refusing it; so is a
            // rummage, which only helps them.
            ChoicePrompt::CostSacrifice
            | ChoicePrompt::CostDiscard
            | ChoicePrompt::CostExile
            | ChoicePrompt::Discard
            | ChoicePrompt::PutBackOnTop
            | ChoicePrompt::PutOnBottom => best(min),
            // "Each player sacrifices a creature" asks with no prompt of its
            // own; its effect says what the answer is.
            ChoicePrompt::Generic
                if context.effects.iter().any(|e| {
                    matches!(
                        e,
                        Effect::SacrificeFilter { .. }
                            | Effect::SacrificeOneThen { .. }
                            | Effect::SacrificeAndIf { .. }
                    )
                }) && options
                    .iter()
                    .all(|id| theirs.battlefield_of(theirs.seat).any(|o| o.id == *id)) =>
            {
                best(min)
            }
            // What they keep, find, take or put down: as little as the
            // question allows, and the least of it.
            ChoicePrompt::Keep
            | ChoicePrompt::KeepLands
            | ChoicePrompt::KeepCreatures
            | ChoicePrompt::KeepPermanents
            | ChoicePrompt::SearchLibrary
            | ChoicePrompt::Wish
            | ChoicePrompt::PutIntoHand
            | ChoicePrompt::PlayFromExile
            | ChoicePrompt::PutOntoBattlefield
            | ChoicePrompt::FromGraveyard
            | ChoicePrompt::OneOfType { .. }
            | ChoicePrompt::PutIntoGraveyard => worst(min),
            // Nothing that helps them: no reveal for an untapped land, no
            // dredge in place of the draw, nothing untapped under Winter Orb.
            ChoicePrompt::RevealOrEnterTapped | ChoicePrompt::Dredge | ChoicePrompt::Untap => {
                worst(min)
            }
            // Separating for this seat's Fact or Fiction: one pile is
            // everything, and this seat takes it.
            ChoicePrompt::FirstPile => options[..usize::from(max).min(options.len())].to_vec(),
            _ => return None,
        })
    }

    /// The targets their spell or ability names, the worst for them first;
    /// `None` where a candidate cannot be read, and for the questions the
    /// ordinary answer reads better: a fight's or a redirect's second word,
    /// a clone's copy, a division of damage.
    #[allow(clippy::too_many_arguments)] // the engine's target offer, plus its explanation
    fn targets_against(
        &self,
        theirs: &PlayerView,
        objects: &[ObjectId],
        players: &[PlayerId],
        min: u32,
        max: u32,
        context: &DecisionContext<'_>,
    ) -> Option<PlayerAction> {
        if context.second_instance
            || context.copying.is_some()
            || context.effects.is_empty()
            || context
                .effects
                .iter()
                .any(|e| matches!(e, Effect::DealDamageDivided { .. }))
        {
            return None;
        }
        let origin = Origin::of(theirs, context.source?);
        let printed = context.printed.map(|p| (p.card(), p.face()));
        // Evenly divided damage hits hardest on one target, as for this
        // seat's own (`HeuristicAgent::choice`).
        let max = if min <= 1
            && context
                .effects
                .iter()
                .any(|e| matches!(e, Effect::DealDamageEvenly { .. }))
        {
            max.min(1)
        } else {
            max
        };
        let mut scored = objects
            .iter()
            .map(|id| Aim::Object(*id))
            .chain(players.iter().map(|p| Aim::Player(*p)))
            .map(|aim| {
                Some((
                    self.aimed_against(theirs, origin, printed, context.effects, aim, context.x)?,
                    aim,
                ))
            })
            .collect::<Option<Vec<_>>>()?;
        scored.sort_by_key(|&(worth, aim)| (worth, aim_key(aim)));
        let wanted = scored
            .iter()
            .take_while(|(worth, _)| *worth < 0)
            .count()
            .clamp(
                usize::try_from(min).unwrap_or(usize::MAX),
                usize::try_from(max).unwrap_or(usize::MAX),
            );
        if wanted > scored.len() {
            return None;
        }
        let mut chosen = Vec::new();
        let mut seats = Vec::new();
        for (_, aim) in scored.into_iter().take(wanted) {
            match aim {
                Aim::Object(id) => chosen.push(id),
                Aim::Player(p) => seats.push(p),
                Aim::Source | Aim::Nothing => {}
            }
        }
        Some(PlayerAction::ChooseTargets {
            objects: chosen,
            players: seats,
        })
    }

    /// The mode of their modal spell or ability worst for them, each at the
    /// worst it can be aimed at; `None` for a cast question that is not a
    /// choice of modes.
    fn mode_against(
        &self,
        theirs: &PlayerView,
        object: ObjectId,
        options: &[CastModeDesc],
    ) -> Option<usize> {
        if let Some(ending) = crate::endings::against(theirs, object, options) {
            return Some(ending);
        }
        let modes = Self::modal_modes(theirs, object)?;
        let origin = Origin::of(theirs, object);
        let worst = |mode: &baylee_cards_dsl::SpellMode| -> Option<i64> {
            if mode.second_targets.is_some() {
                return None;
            }
            self.ends(theirs, origin, None, mode.effects, mode.targets.as_ref(), 0)
                .map(|(low, _)| low)
        };
        options
            .iter()
            .enumerate()
            .filter_map(|(position, option)| {
                let harm = match option.kind {
                    CastModeKind::Mode(i) => worst(modes.get(i)?)?,
                    CastModeKind::Modes(set) => modes
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| set & (1 << i) != 0)
                        .map(|(_, mode)| worst(mode))
                        .sum::<Option<i64>>()?,
                    _ => return None,
                };
                Some((harm, position))
            })
            .min()
            .map(|(_, position)| position)
    }

    /// Their X, against them: as large as their mana pays. What is cast for
    /// them is cast because it hurts them, and a larger X hurts more — with
    /// no ceiling under their library, since a draw that empties it is the
    /// point (CR 704.5b), and a life X up to all of their life (CR 119.4).
    fn number_against(
        &self,
        view: &PlayerView,
        theirs: &PlayerView,
        min: u32,
        max: u32,
        reason: NumberPrompt,
        context: &DecisionContext<'_>,
    ) -> Option<u32> {
        match reason {
            NumberPrompt::X if context.life_x => {
                let life = theirs.seat(theirs.seat)?.life;
                Some(max.min(u32::try_from(life).unwrap_or(0)).max(min))
            }
            NumberPrompt::X => {
                // An X that must match a target's mana value is the
                // ordinary answer's to find.
                if let (Some(source), Some(printed)) = (context.source, context.printed)
                    && context.whole_spell
                    && self
                        .exact_x_values(view, source, printed.card(), printed.face())
                        .is_some()
                {
                    return None;
                }
                Some(Self::largest_x(view, min, max, context, |_| true))
            }
            // Every kick makes their spell better, so they get none.
            NumberPrompt::Multikicker { .. } => Some(min),
            _ => None,
        }
    }

    /// A yes or no of theirs, against them; `None` for the ones the
    /// ordinary answer already gives against them or has no other side.
    fn yes_no_against(
        &self,
        view: &PlayerView,
        theirs: &PlayerView,
        prompt: YesNoPrompt,
        source: Option<baylee_core::ids::AbilityRef>,
        context: &DecisionContext<'_>,
    ) -> Option<bool> {
        Some(match prompt {
            YesNoPrompt::MayDo => self.may_against(theirs, source, context),
            // A pact's debt unpaid loses them the game (CR 724.5: theirs to
            // decide, so this seat's). Their commander stays where it went,
            // and a card of theirs goes to the bottom rather than back as
            // their next draw. And every optional price that buys them
            // something is declined: the life, the energy, the land
            // untapped, the "may" with no name, the creature its sacrifice
            // would have let in.
            YesNoPrompt::PayPact { .. }
            | YesNoPrompt::CommanderZone { .. }
            | YesNoPrompt::CommanderReplace { .. }
            | YesNoPrompt::TopOfLibrary { .. }
            | YesNoPrompt::PayLifeOrEnterTapped { .. }
            | YesNoPrompt::PayLife { .. }
            | YesNoPrompt::PayEnergy { .. }
            | YesNoPrompt::Buyback { .. }
            | YesNoPrompt::EnterTappedForCounters { .. }
            | YesNoPrompt::SacrificeToEnter { .. }
            | YesNoPrompt::Generic => false,
            // A card of theirs cast now, free or not, only where casting it
            // hurts them; a discovered card left uncast goes to their hand,
            // which is less than a free spell.
            YesNoPrompt::Miracle { card }
            | YesNoPrompt::Madness { card }
            | YesNoPrompt::Discover { card }
            | YesNoPrompt::CastWithoutPaying { card }
            | YesNoPrompt::CastPaying { card } => crate::policy::identity(theirs, card)
                .is_some_and(|identity| {
                    self.casting_harm(theirs, card, identity, 0)
                        .is_some_and(|harm| harm < -THRESHOLD)
                }),
            // A tax is paid only where refusing it would counter the spell
            // this seat cast for them; a Rhystic Study's card and Force of
            // Nature's eight damage are theirs to take.
            YesNoPrompt::PayTax { mana } => taxed_against(
                view,
                baylee_core::mana::ManaCost::ZERO.with_more_generic(u32::from(mana)),
                context,
            ),
            YesNoPrompt::PayMana { cost } | YesNoPrompt::PayManaAndLife { cost, .. } => {
                taxed_against(view, cost, context)
            }
            _ => return None,
        })
    }

    /// Their "may": done only when what it does costs them, and never when
    /// it cannot be read.
    fn may_against(
        &self,
        theirs: &PlayerView,
        source: Option<baylee_core::ids::AbilityRef>,
        context: &DecisionContext<'_>,
    ) -> bool {
        // Island Sanctuary: the protection is skipped when they need it and
        // the card when they do not.
        if crate::optional::is_draw_skip(source) {
            return !crate::optional::skips_draw(theirs, |seat| self.hostile(seat, theirs.seat));
        }
        let then = match crate::optional::may_branch(context.effects) {
            Some(Ok(then)) => then,
            None if !context.effects.is_empty() => context.effects,
            Some(Err(())) | None => return false,
        };
        let Some(id) = context.source else {
            return false;
        };
        let aim = theirs
            .object(id)
            .and_then(|o| o.targets.first())
            .and_then(|t| match t {
                baylee_core::ids::TargetRef::Object(r) => Some(r.object),
                baylee_core::ids::TargetRef::Player(_) => None,
            })
            .map_or(Aim::Source, Aim::Object);
        self.effects_worth(theirs, Origin::of(theirs, id), then, aim, context.x)
            .is_some_and(|worth| worth < 0)
    }
}
