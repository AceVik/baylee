//! Hand and mana decisions use legal offers and printed properties. A private
//! scouting summary may adjust values without supplying actionable hidden ids.

use baylee_cards_dsl::{AbilityDef, Effect, FaceDef, Filter, KeywordSet, ManaSource};
use baylee_client_core::manaplan::{self, Source, Tap};
use baylee_core::ids::ObjectId;
use baylee_core::mana::ManaColor;
use baylee_core::preset::HoldUp;
use baylee_core::types::TypeSet;
use baylee_engine::choice::{LegalActions, PlayerAction};
use baylee_engine::engine::DecisionContext;
use baylee_view::{CardIdentity, PlayerView};

use crate::HeuristicAgent;

pub(crate) fn face(card: CardIdentity) -> Option<&'static FaceDef> {
    baylee_cards::by_index(card.index)?
        .faces
        .get(usize::from(card.face))
}

/// The land face of a card held in hand, whichever side it is printed on.
///
/// A [`CardIdentity`] in hand names the face that is *up*, and for a modal
/// double-faced card that is the spell: Shatterskull Smashing is a sorcery
/// with a land on its back, and every reader that asked
/// `types.contains(LAND)` read it as a spell and nothing else. The engine
/// does not — `compute_legal` offers the land drop for a modal card's land
/// back (CR 712.12) — so the agent was refusing to count a card the engine
/// was already offering it.
///
/// The rule is the engine's rule, asked through the same function
/// (`CardDef::land_faces_from_hand`): a modal card's land back is a land
/// drop (CR 712.12) and a transforming card's is not, because in hand it has
/// only its front face's characteristics (CR 712.8a). An agent that
/// disagreed with the offer it is answering would count land drops the
/// engine will not make, or decline ones it is making.
pub(crate) fn land_face(card: CardIdentity) -> Option<&'static FaceDef> {
    let def = baylee_cards::by_index(card.index)?;
    def.land_faces_from_hand()
        .next()
        .and_then(|index| def.faces.get(index))
}

/// Whether a card in hand can be played as a land at all.
pub(crate) fn plays_as_land(card: CardIdentity) -> bool {
    land_face(card).is_some()
}

fn identity(view: &PlayerView, id: ObjectId) -> Option<CardIdentity> {
    view.hand
        .iter()
        .find(|c| c.id == id)
        .map(|c| c.card)
        .or_else(|| view.object(id).and_then(|o| o.card))
}

/// `SplitMix`'s integer finalizer: fixed arithmetic, not a platform hasher or a
/// process RNG. Noise is keyed by the offered object as well as the view.
fn mix(mut n: u64) -> u64 {
    n = (n ^ (n >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    n = (n ^ (n >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    n ^ (n >> 31)
}

impl HeuristicAgent {
    pub(crate) fn miracle(view: &PlayerView, id: ObjectId) -> bool {
        let Some(cost) = identity(view, id).and_then(face).and_then(|f| f.miracle) else {
            return false;
        };
        // The engine asks yes/no inside resolution; it has no intervening
        // priority window in which this controller can tap another source.
        view.seat(view.seat)
            .is_some_and(|s| manaplan::plan(&cost.with_x(0), &s.mana_pool, &[]).is_some())
    }

    pub(crate) fn number(
        &self,
        view: &PlayerView,
        min: u32,
        max: u32,
        context: &baylee_engine::engine::DecisionContext<'_>,
    ) -> u32 {
        let Some(seat) = view.seat(view.seat) else {
            return min;
        };
        if context.life_x {
            // Pay the smallest amount giving the best exchange; preserving
            // life and friendly creatures matters more than maximizing X.
            return (min..=max.min(u32::try_from(seat.life.saturating_sub(1)).unwrap_or(0)))
                .max_by_key(|&x| {
                    let material: i64 = view
                        .battlefield
                        .iter()
                        .filter(|o| {
                            o.types.contains(TypeSet::CREATURE)
                                && o.toughness.is_some_and(|t| i64::from(t) <= i64::from(x))
                        })
                        .map(|o| {
                            crate::tactics::material(o)
                                * if self.hostile(o.controller, view.seat) {
                                    1
                                } else {
                                    -1
                                }
                        })
                        .sum();
                    (material - i64::from(x) * 60, std::cmp::Reverse(x))
                })
                .unwrap_or(min);
        }
        let Some(cost) = context
            .cost
            .filter(baylee_core::mana::ManaCost::has_variable)
        else {
            return min;
        };
        (min..=max.min(context.x_targets.unwrap_or(max)))
            .rev()
            .find(|&x| {
                manaplan::plan(&cost.with_x(x), &seat.mana_pool, &[]).is_some()
                    && crate::tactics::meaning(context.effects, x).draws(view) < seat.library_count
            })
            .unwrap_or(min)
    }

    pub(crate) fn noise(&self, view: &PlayerView, id: ObjectId) -> i64 {
        let width = u64::from(self.profile.temperature_milli.min(10_000));
        if width == 0 {
            return 0;
        }
        let seed = self.seed ^ view.seq ^ (u64::from(id.slot()) << 16) ^ u64::from(view.seat.get());
        i64::try_from(mix(seed) % (2 * width + 1)).unwrap_or(0) - i64::try_from(width).unwrap_or(0)
    }

    pub(crate) fn mulligan(&self, view: &PlayerView, taken: u8, free: bool) -> PlayerAction {
        let skill = self.profile.mulligan_skill;
        // Four cards can still play; repeatedly demanding an ideal seven
        // would mulligan a land-light deck to zero.
        let limit = if free { 4 } else { 3 };
        if skill == 0 || taken >= limit || view.hand.len() < 5 {
            return PlayerAction::MulliganKeep;
        }
        let lands = view.hand.iter().filter(|c| plays_as_land(c.card)).count();
        let balance = (2..=5).contains(&lands);
        let curve = skill < 2
            || view.hand.iter().any(|card| {
                !plays_as_land(card.card)
                    && card.mana_value <= 3
                    && face(card.card).is_some_and(|f| {
                        let colors = view.hand.iter().filter_map(|c| land_face(c.card)).fold(
                            baylee_core::color::ColorSet::EMPTY,
                            |colors, land| {
                                let mut subtypes = baylee_core::types::SubtypeSet::EMPTY;
                                for &subtype in land.subtypes {
                                    subtypes.insert(subtype);
                                }
                                let color = manaplan::basic_land_color(&subtypes);
                                color.map_or(colors, |color| colors.union(color_set(color)))
                            },
                        );
                        // Nonbasic colour sources may need board context. Do not
                        // reject an otherwise playable hand on an unknown colour.
                        f.mana_cost.colors().is_empty()
                            || f.mana_cost.colors().intersects(colors)
                            || view
                                .hand
                                .iter()
                                .any(|c| land_face(c.card).is_some_and(|l| l.subtypes.is_empty()))
                    })
            });
        if balance && (curve || taken >= 2) {
            PlayerAction::MulliganKeep
        } else {
            PlayerAction::MulliganTake
        }
    }

    pub(crate) fn discard(&self, view: &PlayerView, count: usize) -> Vec<ObjectId> {
        if self.profile.mulligan_skill == 0 {
            return crate::costliest(view, count);
        }
        let mut hand: Vec<_> = view.hand.iter().collect();
        let mut lands = hand.iter().filter(|c| plays_as_land(c.card)).count();
        let mut result = Vec::with_capacity(count);
        for _ in 0..count.min(hand.len()) {
            let worst = hand
                .iter()
                .enumerate()
                .min_by_key(|(_, c)| {
                    let value = if plays_as_land(c.card) {
                        if lands > 3 { -1000 } else { 2000 }
                    } else {
                        700 - i64::from(c.mana_value) * 100
                    };
                    (value, c.id)
                })
                .map_or(0, |(i, _)| i);
            let card = hand.remove(worst);
            if plays_as_land(card.card) {
                lands = lands.saturating_sub(1);
            }
            result.push(card.id);
        }
        result
    }

    pub(crate) fn select_cards(
        &self,
        view: &PlayerView,
        options: &[ObjectId],
        min: u8,
        max: u8,
        prompt: baylee_engine::choice::ChoicePrompt,
    ) -> Option<Vec<ObjectId>> {
        use baylee_engine::choice::ChoicePrompt;
        // Ahead of the skill gate, because this one needs no skill: the card
        // never leaves the hand (CR 701.20b), nothing is spent, and the only
        // thing given up by revealing is the information. A profile that
        // fell through to the `min` default below would answer zero and play
        // every reveal land tapped for the whole game.
        if prompt == ChoicePrompt::RevealOrEnterTapped {
            let mut ranked = options.to_vec();
            ranked.sort_unstable();
            ranked.truncate(usize::from(max));
            return Some(ranked);
        }
        if self.profile.mulligan_skill < 2 {
            return None;
        }
        let value = Self::card_value(view);
        let mut ranked = options.to_vec();
        let count = match prompt {
            ChoicePrompt::SearchLibrary | ChoicePrompt::Wish => {
                ranked.sort_by_key(|id| (std::cmp::Reverse(value(id)), *id));
                usize::from(max)
            }
            // A price, paid with the least valuable card on the menu — and
            // paid at all, which `min` would not do: "… unless you sacrifice
            // a creature" asks with `min: 0` because naming nothing *is* the
            // refusal, and an agent answering `min` there lost the permanent
            // the price was protecting, every time and silently. An
            // activation asks with `min: 1`, where this is the same answer.
            ChoicePrompt::CostSacrifice | ChoicePrompt::CostDiscard | ChoicePrompt::CostExile => {
                ranked.sort_by_key(|id| (value(id), *id));
                usize::from(min.max(1).min(max))
            }
            // Not a price: `Effect::PutFromHandOnTop` asks with
            // `min == max`, so `min` is the whole answer.
            ChoicePrompt::PutBackOnTop => {
                ranked.sort_by_key(|id| (value(id), *id));
                usize::from(min)
            }
            _ => return None,
        };
        ranked.truncate(count);
        Some(ranked)
    }

    /// The looked-at cards a scry sends to the bottom or a surveil into the
    /// graveyard, worst first; `None` below the skill that reads cards at
    /// all.
    ///
    /// Only a card worth less than nothing goes: unknown cards are kept,
    /// because an id alone is not information about the top of a library.
    pub(crate) fn send_away(&self, view: &PlayerView, cards: &[ObjectId]) -> Option<Vec<ObjectId>> {
        if self.profile.mulligan_skill < 2 {
            return None;
        }
        let value = Self::card_value(view);
        let mut away: Vec<ObjectId> = cards.iter().copied().filter(|id| value(id) < 0).collect();
        away.sort_by_key(|id| (value(id), *id));
        Some(away)
    }

    /// What a card is worth to this seat right now: a land by how many it
    /// already has, a spell by how far its mana value is out of reach. Zero
    /// for a card whose identity this seat cannot see.
    fn card_value(view: &PlayerView) -> impl Fn(&ObjectId) -> i64 + '_ {
        let lands = view
            .battlefield_of(view.seat)
            .filter(|o| o.types.contains(TypeSet::LAND))
            .count();
        move |id: &ObjectId| -> i64 {
            let Some(card) = identity(view, *id) else {
                return 0;
            };
            let Some(f) = face(card) else {
                return 0;
            };
            if f.types.contains(TypeSet::LAND) {
                if lands < 4 {
                    2000
                } else if lands >= 7 {
                    -100
                } else {
                    200
                }
            } else {
                800 - i64::from(
                    f.mana_cost
                        .cmc()
                        .saturating_sub(u32::try_from(lands).unwrap_or(u32::MAX)),
                ) * 150
            }
        }
    }

    /// The colour to name for mana being made. Restricted mana (CR 106.6) is
    /// named for the spells it may pay for and no others: Ancient Ziggurat's
    /// named white for the Swords to Plowshares beside the Birds it was
    /// tapped for would pay for neither.
    pub(crate) fn color(
        &self,
        view: &PlayerView,
        options: &[ManaColor],
        context: &DecisionContext<'_>,
    ) -> ManaColor {
        let only_for = crate::restricted::only_for(context.effects);
        let may_pay = |id: ObjectId| {
            only_for.is_none_or(|filter| self.admits(view, filter, context.source, id))
        };
        if self.profile.mulligan_skill >= 2
            && let Some(color) = self.planned_color(view, options, &may_pay)
        {
            return color;
        }
        options
            .iter()
            .copied()
            .max_by_key(|&color| {
                let demand: u32 = view
                    .hand
                    .iter()
                    .map(|c| (c.id, Some(c.card)))
                    .chain(
                        view.command
                            .get(usize::from(view.seat.get()))
                            .into_iter()
                            .flatten()
                            .filter(|o| o.commander)
                            .map(|o| (o.id, o.card)),
                    )
                    .filter(|&(id, _)| may_pay(id))
                    .filter_map(|(_, card)| card.and_then(face))
                    .map(|f| {
                        f.mana_cost
                            .symbols()
                            .filter(|s| s.colors().intersects(color_set(color)))
                            .count() as u32
                    })
                    .sum();
                let floating = view.seat(view.seat).map_or(0, |s| {
                    baylee_client_core::manapool::plain(&s.mana_pool, color)
                });
                (
                    i64::from(demand) * 100 - i64::from(floating) * 150,
                    std::cmp::Reverse(color.index()),
                )
            })
            .unwrap_or(ManaColor::Colorless)
    }

    /// Reconstruct a useful payment from this view instead of remembering a
    /// previous decision. The mana choice has no legal-action offer attached;
    /// projected untapped sources are estimates here, never actions to send.
    fn planned_color(
        &self,
        view: &PlayerView,
        options: &[ManaColor],
        may_pay: &impl Fn(ObjectId) -> bool,
    ) -> Option<ManaColor> {
        let sources = remaining_sources(view);
        let seat = view.seat(view.seat)?;
        let mut best = None;
        for &color in options {
            let mut pool = seat.mana_pool;
            // The prompt does not carry an amount. One unit is a conservative
            // estimate; variable or multi-mana production may do better.
            match color {
                ManaColor::White => pool.white += 1,
                ManaColor::Blue => pool.blue += 1,
                ManaColor::Black => pool.black += 1,
                ManaColor::Red => pool.red += 1,
                ManaColor::Green => pool.green += 1,
                ManaColor::Colorless => pool.colorless += 1,
            }
            for id in view.hand.iter().map(|c| c.id).chain(
                view.command
                    .get(usize::from(view.seat.get()))
                    .into_iter()
                    .flatten()
                    .filter(|o| o.commander)
                    .map(|o| o.id),
            ) {
                if !may_pay(id) {
                    continue;
                }
                let Some(card) = identity(view, id) else {
                    continue;
                };
                let Some(f) = face(card) else { continue };
                if f.types.contains(TypeSet::LAND) {
                    continue;
                }
                let score = self.spell_score(view, card) + self.noise(view, id);
                if score <= 0 {
                    continue;
                }
                let cost = spell_cost(view, id, f);
                let Some(plan) = manaplan::plan(&cost, &pool, &sources) else {
                    continue;
                };
                let quality = (
                    score,
                    std::cmp::Reverse(plan.taps()),
                    std::cmp::Reverse(color.index()),
                );
                if best.as_ref().is_none_or(|(old, _)| quality > *old) {
                    best = Some((quality, color));
                }
            }
        }
        best.map(|(_, color)| color)
    }

    /// What may pay for `spell` when a restricted tap can (CR 106.6): the
    /// unrestricted sources, with the best restricted tap whose filter admits
    /// it in place of whatever else that permanent makes; and that tap.
    ///
    /// One restricted tap and not every one: after the first, the next plan
    /// cannot see its mana floating (`restricted`).
    fn paying_for(
        &self,
        view: &PlayerView,
        restricted: &[Offer],
        plain: &[Source],
        spell: ObjectId,
    ) -> Option<(Vec<Source>, Source)> {
        let best = restricted
            .iter()
            .filter(|o| {
                o.only_for
                    .is_some_and(|filter| self.admits(view, filter, Some(o.source.id), spell))
            })
            .min_by_key(|o| {
                (
                    o.source.priced,
                    std::cmp::Reverse(o.source.amount),
                    std::cmp::Reverse(o.source.colors.len()),
                    o.source.id,
                )
            })?;
        let mut sources: Vec<Source> = plain
            .iter()
            .filter(|s| s.id != best.source.id)
            .cloned()
            .collect();
        // Last, because `manaplan::plan` sends its steps in source order and
        // restricted mana has to be the last tap: once it floats, no plan
        // counts it, and the engine's own merge makes the spell castable only
        // when the rest is already there.
        sources.push(best.source.clone());
        Some((sources, best.source.clone()))
    }

    /// Every candidate is paid for before a tap is committed. A five-mana
    /// card beside two lands used to make the agent tap both, then pass.
    pub(crate) fn spell_or_mana(
        &self,
        view: &PlayerView,
        legal: &LegalActions,
    ) -> Option<PlayerAction> {
        let seat = view.seat(view.seat)?;
        let pool = &seat.mana_pool;
        let command = view.command.get(usize::from(view.seat.get()));
        let (restricted, plain) = split_restricted(offers(view, legal));
        let plain = usable(plain);
        let main = view.active == view.seat
            && view.stack.is_empty()
            && matches!(
                view.phase,
                baylee_view::Phase::FirstMain | baylee_view::Phase::SecondMain
            );
        let available = pool.total() + plain.iter().map(|s| u32::from(s.amount)).sum::<u32>();
        let reserve = self.reserve(view, main);
        let mut best: Option<(i64, PlayerAction)> = None;
        for id in view
            .hand
            .iter()
            .map(|c| c.id)
            .chain(
                command
                    .into_iter()
                    .flatten()
                    .filter(|o| o.commander)
                    .map(|o| o.id),
            )
            .chain(legal.castable.iter().copied())
        {
            let Some(card) = identity(view, id) else {
                continue;
            };
            let Some(f) = face(card) else {
                continue;
            };
            if f.types.contains(TypeSet::LAND) {
                continue;
            }
            let instant = f.types.contains(TypeSet::INSTANT)
                || baylee_cards::by_index(card.index).is_some_and(|d| {
                    d.keywords_for_face(usize::from(card.face))
                        .contains(KeywordSet::FLASH)
                });
            if !legal.castable.contains(&id) && (!main && !instant || !f.castable_from_hand) {
                continue;
            }
            if view.sorcery_lock.is_some() && !main {
                continue;
            }
            let score = self.spell_score(view, card) + self.noise(view, id);
            if score <= 0 {
                continue;
            }
            // What the reserve leaves to spend on this spell.
            let budget = if main && !instant {
                available.saturating_sub(reserve)
            } else {
                available
            };
            let (cost, floats_first) = aim(view, card, f, spell_cost(view, id, f), &plain, budget);
            // A price floated before the cast is checked against the pool the
            // planner reads, which holds no restricted mana (`restricted`).
            let paying = if floats_first || restricted.is_empty() {
                None
            } else {
                self.paying_for(view, &restricted, &plain, id)
            };
            let (sources, last_tap) = paying
                .as_ref()
                .map_or((&plain[..], None), |(all, only)| (&all[..], Some(only)));
            let action = if legal.castable.contains(&id)
                && (!floats_first || manaplan::plan(&cost, pool, &[]).is_some())
            {
                PlayerAction::CastSpell { card: id }
            } else {
                let Some(plan) = manaplan::plan(&cost, pool, sources) else {
                    continue;
                };
                let Some(step) = plan.steps.first() else {
                    continue;
                };
                match step.tap {
                    Tap::Intrinsic => PlayerAction::ActivateManaAbility {
                        source: step.source,
                    },
                    Tap::Ability(ability_index) => PlayerAction::ActivateAbility {
                        source: step.source,
                        ability_index,
                    },
                }
            };
            // Establish a board before reserving answers. Once a creature is
            // out, holding the last two mana can protect the investment.
            // Restricted mana pays for this spell first and could hold up
            // nothing else.
            let spent = cost
                .cmc()
                .saturating_sub(last_tap.map_or(0, |r| u32::from(r.amount)));
            if main && !instant && available.saturating_sub(spent) < reserve {
                continue;
            }
            if best.as_ref().is_none_or(|(value, _)| score > *value) {
                best = Some((score, action));
            }
        }
        best.map(|(_, action)| action)
    }

    fn reserve(&self, view: &PlayerView, main: bool) -> u32 {
        if !main
            || self.profile.hold_up == HoldUp::None
            || !view
                .battlefield_of(view.seat)
                .any(|o| o.types.contains(TypeSet::CREATURE))
        {
            return 0;
        }
        if self.strategy.scouted_opponents
            && !self.strategy.known_threat
            && !view.seats.iter().any(|s| {
                self.hostile(s.player, view.seat) && crate::board_pressure(view, s.player) > 2
            })
        {
            return 0;
        }
        if self.profile.hold_up == HoldUp::ThreatAware
            && !view.seats.iter().any(|s| {
                self.hostile(s.player, view.seat)
                    && (s.hand_count > 0 || crate::board_pressure(view, s.player) > 2)
            })
        {
            return 0;
        }
        view.hand
            .iter()
            .filter(|c| c.types.contains(TypeSet::INSTANT))
            .filter(|c| face(c.card).is_some_and(|f| f.mana_cost.cmc() <= 3))
            .map(|c| c.mana_value)
            .filter(|&v| v > 0)
            .min()
            .unwrap_or(0)
    }

    pub(crate) fn spell_score(&self, view: &PlayerView, card: CardIdentity) -> i64 {
        let Some(f) = face(card) else {
            return 0;
        };
        let Some(def) = baylee_cards::by_index(card.index) else {
            return 0;
        };
        let spells = def.abilities_for_face(usize::from(card.face));
        let effects = spells.iter().filter_map(|a| match a {
            AbilityDef::Spell { effects, .. } => Some(*effects),
            _ => None,
        });
        let enemy_board = view
            .battlefield
            .iter()
            .any(|o| self.hostile(o.controller, view.seat) && !o.types.contains(TypeSet::LAND));
        let mut value = 200 + i64::from(f.mana_cost.cmc()) * 100;
        if f.types.contains(TypeSet::CREATURE) {
            value += 250 + i64::from(f.power.unwrap_or(0)) * 60 + self.strategy.creature_bonus;
            if self.strategy.sweeper_risk
                && view
                    .battlefield_of(view.seat)
                    .filter(|o| o.types.contains(TypeSet::CREATURE))
                    .count()
                    >= 2
            {
                value -= 900;
            }
        }
        for effect in effects.flatten() {
            // A deferred loss is not a free counter. The current view has no
            // future payment window to plan against; decline an obligation
            // this stateless policy cannot discharge.
            if matches!(effect, Effect::PayCostOrLoseLater { .. }) {
                return -10_000;
            }
            if counter(effect)
                && !view.stack.iter().any(|o| {
                    self.hostile(o.controller, view.seat)
                        && match effect {
                            Effect::CounterTargetSpell | Effect::CounterTargetSpellToExile => {
                                !matches!(
                                    o.stack_item,
                                    Some(baylee_view::StackItem::Ability { .. })
                                )
                            }
                            Effect::CounterTargetAbility => {
                                matches!(o.stack_item, Some(baylee_view::StackItem::Ability { .. }))
                            }
                            _ => true,
                        }
                })
            {
                return -10_000;
            }
            if removal(effect) && !enemy_board {
                return -10_000;
            }
            if counter(effect) {
                value += 1500 + self.strategy.interaction_bonus;
            }
            if removal(effect) {
                value += 400 + self.strategy.interaction_bonus;
            }
            if matches!(
                effect,
                Effect::DrawCards { .. }
                    | Effect::DrawCardsFor { .. }
                    | Effect::LookAtTopPick { .. }
            ) {
                value += 300 + self.strategy.draw_bonus;
            }
        }
        if f.types.contains(TypeSet::ARTIFACT) {
            value += self.strategy.artifact_bonus;
        }
        value
    }
}

fn color_set(color: ManaColor) -> baylee_core::color::ColorSet {
    use baylee_core::color::{Color, ColorSet};
    match color {
        ManaColor::White => ColorSet::of(Color::White),
        ManaColor::Blue => ColorSet::of(Color::Blue),
        ManaColor::Black => ColorSet::of(Color::Black),
        ManaColor::Red => ColorSet::of(Color::Red),
        ManaColor::Green => ColorSet::of(Color::Green),
        ManaColor::Colorless => ColorSet::EMPTY,
    }
}

fn counter(effect: &Effect) -> bool {
    matches!(
        effect,
        Effect::CounterTargetSpell
            | Effect::CounterTargetSpellToExile
            | Effect::CounterTargetAbility
            | Effect::CounterTargetSpellOrAbility
    )
}
fn removal(effect: &Effect) -> bool {
    matches!(
        effect,
        Effect::Destroy { .. } | Effect::Exile { .. } | Effect::PutTargetOnBottomOfLibrary
    )
}

/// One tap toward what this seat owes inside a payment window (CR 605.3a).
///
/// The window is an ordinary priority round, which is exactly what made it
/// invisible. A seat that has just agreed to pay ward's tax is handed
/// priority over untapped lands with **nothing castable** — the tax is not a
/// spell, so [`Policy::spell_or_mana`] finds no candidate to pay for and
/// every path below it passes. The agent said yes and then lost the spell
/// anyway, which is worse than having said no. `PlayerView::owed` is the
/// engine saying the price out loud, and this is its only reader.
///
/// **Nothing is tapped toward a price the seat cannot finish.**
/// [`manaplan::plan`] answers `None` when no assignment of the offered
/// sources covers the whole cost, and `None` here means pass: a seat that
/// taps two of the three lands it needs has lost the mana *and* the spell,
/// where a seat that taps none has lost only what it had already agreed to
/// lose. The refusal is therefore before the first tap and not after it.
///
/// It terminates on its own. `owed` is the total that was asked rather than
/// the remainder, and `plan` spends the floating pool first — so each tap
/// leaves one fewer step, and a pool that covers the price plans no steps at
/// all, which is the pass that closes the window.
pub(crate) fn pay_owed(view: &PlayerView, legal: &LegalActions) -> Option<PlayerAction> {
    let owed = view.owed?;
    // `owed` is what the *awaited* seat owes, and that is only this seat
    // while this seat is the one being asked. Both fields ride in every
    // view, so reading one without the other would have a seat paying for
    // somebody else's window.
    if view.awaiting != Some(view.seat) {
        return None;
    }
    let seat = view.seat(view.seat)?;
    let plan = manaplan::plan(&owed, &seat.mana_pool, &sources(view, legal))?;
    let step = plan.steps.first()?;
    Some(match step.tap {
        Tap::Intrinsic => PlayerAction::ActivateManaAbility {
            source: step.source,
        },
        Tap::Ability(ability_index) => PlayerAction::ActivateAbility {
            source: step.source,
            ability_index,
        },
    })
}

/// Whether an ability charges anything beyond tapping the permanent.
///
/// `mana_shape` has already refused a **mana** price (`cost.mana` must be
/// `ZERO`), so what is left is `cost.parts` — the half it never looks at, and
/// the half that sells the land. `{T}` is the only part a mana source may
/// carry for free, so this is a `matches!` on that one variant rather than a
/// list of the expensive ones: a `CostPart` added tomorrow is priced by
/// default, which is the direction that cannot go quiet. `{Q}` is not on the
/// free side either — untapping a permanent is a different button from
/// tapping it, whatever mana comes out.
fn priced(cost: &baylee_cards_dsl::Cost) -> bool {
    cost.parts
        .iter()
        .any(|part| !matches!(part, baylee_cards_dsl::CostPart::TapSelf))
}

/// Read only offered, simple mana taps. One permanent is one source even
/// when its intrinsic and printed abilities both appear in the offer.
///
/// **What a tap costs is part of the ranking**, and it was not. The dedup
/// below is the only thing in either planner enforcing "one permanent taps
/// once" — nothing under `manaplan::plan` keys on `ObjectId` — so the entry
/// that survives it is the only mode the agent will ever use for that
/// permanent, and the key decided that on mana made and colours reached.
/// Havenwood Battleground prints `{T}: Add {G}` beside `{T}, Sacrifice this
/// land: Add {G}{G}`, so `Reverse(amount)` kept the sacrifice and threw the
/// free tap away: the agent sold the land for a mana it already had. Spire of
/// Industry is the same shape one column later — equal amounts, five colours
/// against one — and paid the life whenever colourless was what the plan
/// asked for.
///
/// So `priced` sorts **before** the amount. Free first, and among free modes
/// the old order is untouched. A permanent whose *only* mana ability is
/// priced is unaffected: the dedup keeps one entry per permanent whatever the
/// key says, so this changes which mode survives and never how many.
///
/// Restricted taps are left out: what their mana may pay for is a question
/// about one spell, which [`HeuristicAgent::paying_for`] asks.
fn sources(view: &PlayerView, legal: &LegalActions) -> Vec<Source> {
    usable(split_restricted(offers(view, legal)).1)
}

/// The restricted taps and the rest. Moved out only when there are any: this
/// runs at every priority, and a board without restricted mana is the usual
/// one.
fn split_restricted(mut offers: Vec<Offer>) -> (Vec<Offer>, Vec<Offer>) {
    if offers.iter().all(|o| o.only_for.is_none()) {
        return (Vec::new(), offers);
    }
    let restricted = offers.extract_if(.., |o| o.only_for.is_some()).collect();
    (restricted, offers)
}

/// One offered mana tap, before the one-permanent-one-source dedup.
struct Offer {
    source: Source,
    /// What its mana may be spent on, when it is restricted (CR 106.6).
    only_for: Option<&'static Filter>,
}

/// Every offered simple mana tap, restricted ones with their filter.
fn offers(view: &PlayerView, legal: &LegalActions) -> Vec<Offer> {
    // The price rides on `manaplan::Source` itself. It used to be paired
    // with the source only until the dedup had run, because the solver asked
    // nothing but what comes out; since #165's second half the solver ranks
    // whole permanents by it too, and reaches for a priced tap only where no
    // clean one fits the pip.
    //
    // **`bundle` on `Source` is not this flag.** `priced` is what the tap
    // *costs*; `bundle` is how `colors` is *read* — one of each, or a choice
    // of one. Folding them compiles and passes every test on either side,
    // because the client's tests build a `Source` directly and never come
    // through here. Three readers currently agree about the same family of
    // lands from three directions — `mana_bundle`, `duplicates_intrinsic`
    // and this field — and one flag doing two jobs would silently make that
    // one reader wearing three names.
    let mut result: Vec<Offer> = Vec::new();
    for &id in &legal.mana_abilities {
        if let Some(color) = view
            .object(id)
            .and_then(|o| manaplan::basic_land_color(&o.subtypes))
        {
            // CR 305.6: the intrinsic tap of a basic land type costs the tap
            // and nothing else.
            result.push(Offer {
                source: Source::fixed(id, Tap::Intrinsic, color),
                only_for: None,
            });
        }
    }
    for &(id, index) in &legal.abilities {
        let Some(object) = view.object(id) else {
            continue;
        };
        let source = if let Some(slot) = baylee_engine::choice::granted_slot(index) {
            object
                .granted_mana
                .as_ref()
                .filter(|m| m.slot == slot)
                // A granted ability is free by construction: `GrantedMana`
                // carries colours and an amount and no cost at all, and the
                // registry lookup below would be reaching for a printed
                // ability that has nothing to do with the grant.
                .map(|m| (m.colors.clone(), m.amount, false, None))
        } else {
            crate::activate::printed(view, id, index).and_then(|a| {
                let (AbilityDef::Activated {
                    cost,
                    effects,
                    mana_ability: true,
                    ..
                }
                | AbilityDef::ActivatedConditional {
                    cost,
                    effects,
                    mana_ability: true,
                    ..
                }) = a
                else {
                    return None;
                };
                let (kind, amount, _) = baylee_cards_dsl::mana_shape(cost, effects)?;
                let colors = match kind {
                    ManaSource::Fixed(c) => vec![c],
                    ManaSource::Choice(c) => c.to_vec(),
                    _ => object
                        .board_mana
                        .as_ref()
                        .filter(|m| m.index == index)?
                        .colors
                        .clone(),
                };
                Some((
                    colors,
                    amount?,
                    priced(cost),
                    crate::restricted::only_for(effects),
                ))
            })
        };
        if let Some((colors, amount, priced, only_for)) = source {
            result.push(Offer {
                source: Source {
                    id,
                    tap: Tap::Ability(index),
                    colors,
                    amount,
                    // Correct only while `mana_shape`'s one-effect match hides
                    // every multi-`AddMana` ability from this reader, so nothing
                    // that reaches here is a bundle. #170 is where that stops
                    // being true, and it has to decide this per ability rather
                    // than restate the constant.
                    bundle: false,
                    priced,
                },
                only_for,
            });
        }
    }
    result
}

/// One source per permanent, the mode [`sources`] explains.
fn usable(offers: Vec<Offer>) -> Vec<Source> {
    let mut result: Vec<Source> = offers.into_iter().map(|o| o.source).collect();
    result.sort_by_key(|s| {
        (
            s.id,
            s.priced,
            std::cmp::Reverse(s.amount),
            std::cmp::Reverse(s.colors.len()),
            matches!(s.tap, Tap::Ability(_)),
        )
    });
    result.dedup_by_key(|s| s.id);
    result
}

/// Costs visible from the card and public commander/graveyard bookkeeping.
fn spell_cost(view: &PlayerView, id: ObjectId, face: &FaceDef) -> baylee_core::mana::ManaCost {
    let mut cost = face.mana_cost;
    if view
        .command
        .get(usize::from(view.seat.get()))
        .is_some_and(|cards| cards.iter().any(|o| o.id == id))
    {
        let casts = view
            .seat(view.seat)
            .and_then(|s| s.commanders.iter().find(|c| c.object == id))
            .map_or(0, |c| c.casts);
        cost = cost.with_more_generic(casts.saturating_mul(2));
    }
    if face.delve {
        let grave = view
            .graveyards
            .get(usize::from(view.seat.get()))
            .map_or(0, Vec::len);
        cost = cost.with_less_generic(u32::try_from(grave).unwrap_or(u32::MAX));
    }
    cost
}

/// The price a spell is planned toward, and whether it has to be floating
/// before the cast. X is the largest the sources can make. A kicker is
/// aimed for when it fits the budget, and cast only once floating, as with X:
/// the engine asks the kicker question with no window to tap anything more.
fn aim(
    view: &PlayerView,
    card: CardIdentity,
    face: &FaceDef,
    cost: baylee_core::mana::ManaCost,
    sources: &[Source],
    budget: u32,
) -> (baylee_core::mana::ManaCost, bool) {
    let Some(pool) = view.seat(view.seat).map(|s| &s.mana_pool) else {
        return (cost, false);
    };
    if cost.has_variable() {
        let available = pool.total() + sources.iter().map(|s| u32::from(s.amount)).sum::<u32>();
        let x = (1..=available.min(50))
            .rev()
            .find_map(|x| {
                let cost = cost.with_x(x);
                manaplan::plan(&cost, pool, sources)
                    .is_some()
                    .then_some(cost)
            })
            .unwrap_or_else(|| cost.with_x(0));
        return (x, true);
    }
    kicked_price(view, face, spell_effects(card), cost, sources)
        .filter(|net| net.cmc() <= budget && manaplan::plan(net, pool, sources).is_some())
        .map_or((cost, false), |net| (net, true))
}

/// The effects of the spell ability on the face `card` names.
fn spell_effects(card: CardIdentity) -> &'static [Effect] {
    baylee_cards::by_index(card.index)
        .and_then(|d| {
            d.abilities_for_face(usize::from(card.face))
                .iter()
                .find_map(|a| match a {
                    AbilityDef::Spell { effects, .. } => Some(*effects),
                    _ => None,
                })
        })
        .unwrap_or(&[])
}

/// What `cost` becomes with the face's optional additional costs paid, less
/// what the convoke-style help will pay; `None` when they are not to be paid.
///
/// Kicker (CR 702.33a) and "you may waterbend" (CR 701.67a) are one question
/// to the engine, `YesNoPrompt::Kicker`, and one answer here, asked twice: by
/// the planner, so the mana is floating before the cast, and by the question
/// itself, which the engine pays out of the pool alone — a yes the pool
/// cannot cover loses the whole cast (CR 601.2h), where a no loses only the
/// bonus. The kicked half is the better one by design, so it is paid whenever
/// it can be, unless it would draw the library out.
///
/// The help is every untapped creature and artifact not already counted as a
/// mana source, because the convoke question is answered by tapping all of
/// them. A cost with a non-mana part is never paid: nothing reads one yet.
fn kicked_price(
    view: &PlayerView,
    face: &FaceDef,
    effects: &[Effect],
    cost: baylee_core::mana::ManaCost,
    sources: &[Source],
) -> Option<baylee_core::mana::ManaCost> {
    let extra = face.additional_costs;
    if extra.is_empty() || extra.iter().any(|c| !c.parts.is_empty()) {
        return None;
    }
    let library = view.seat(view.seat)?.library_count;
    let draws: u32 = effects
        .iter()
        .map(|effect| match effect {
            Effect::IfKicked { then, .. } => crate::tactics::meaning(then, 0).draws(view),
            other => crate::tactics::meaning(std::slice::from_ref(other), 0).draws(view),
        })
        .sum();
    if draws >= library {
        return None;
    }
    let total = extra.iter().fold(cost, |total, c| total.combine(&c.mana));
    let help = if face.convoke {
        view.battlefield_of(view.seat)
            .filter(|o| {
                o.types
                    .intersects(TypeSet::CREATURE.union(TypeSet::ARTIFACT))
            })
            .filter(|o| {
                !o.status.contains(baylee_view::ObjectStatus::TAPPED)
                    && !o.status.contains(baylee_view::ObjectStatus::PHASED_OUT)
            })
            .filter(|o| sources.iter().all(|s| s.id != o.id))
            .count()
    } else {
        0
    };
    Some(total.with_less_generic(u32::try_from(help).unwrap_or(u32::MAX)))
}

/// The answer to `YesNoPrompt::Kicker`: yes when the floating pool covers the
/// kicked price, which [`kicked_price`] says is the only safe yes.
pub(crate) fn kicks(
    view: &PlayerView,
    context: &baylee_engine::engine::DecisionContext<'_>,
) -> bool {
    let (Some(id), Some(cost), Some(seat)) = (context.source, context.cost, view.seat(view.seat))
    else {
        return false;
    };
    identity(view, id)
        .and_then(face)
        .and_then(|f| kicked_price(view, f, context.effects, cost.with_x(context.x), &[]))
        .is_some_and(|net| manaplan::plan(&net, &seat.mana_pool, &[]).is_some())
}

/// Whether a tax is worth paying, asked of what refusing it would do.
///
/// `YesNoPrompt::PayTax` carries a price and not a consequence, and the two
/// taxes in this pool are not the same decision. Ward (CR 702.21) reaches
/// the **caster** and counters the spell already on the stack, so refusing
/// it throws away a whole card to keep two mana. A Rhystic tax gives an
/// opponent one card, and that one stays refused: a stateless policy cannot
/// tell mana it has to spare from mana its own curve needs this turn, and a
/// card is the cheaper of the two to give up. The resolving effect is what
/// separates them, and `decision_context` carries it here because the
/// operation that asked has not advanced past itself yet.
///
/// Affordability is the second half and not a formality. A seat that says
/// yes with an empty pool is handed a mana window under CR 605.3a, and a
/// window it cannot fill ends exactly where refusing ended — one question
/// later.
pub(crate) fn pays_tax(
    view: &PlayerView,
    mana: u16,
    context: &baylee_engine::engine::DecisionContext<'_>,
) -> bool {
    let refusal_counters = context.effects.iter().any(|effect| match effect {
        Effect::PlayerMayPayOr { effect, .. } => matches!(
            effect,
            Effect::CounterTargetSpell
                | Effect::CounterTargetSpellToExile
                | Effect::CounterTargetAbility
                | Effect::CounterTargetSpellOrAbility
        ),
        _ => false,
    });
    if !refusal_counters {
        return false;
    }
    can_pay(
        view,
        &baylee_core::mana::ManaCost::ZERO.with_more_generic(u32::from(mana)),
    )
}

/// Whether the seat could produce `cost` right now, floating mana plus what
/// is still untapped.
///
/// An estimate, like [`remaining_sources`] it is built on, and deliberately
/// so: the engine's own offer decides what may actually be tapped, and this
/// is asked at moments — choosing a target, answering a tax — where no offer
/// exists yet.
pub(crate) fn can_pay(view: &PlayerView, cost: &baylee_core::mana::ManaCost) -> bool {
    view.seat(view.seat).is_some_and(|seat| {
        manaplan::plan(cost, &seat.mana_pool, &remaining_sources(view)).is_some()
    })
}

/// Only for evaluating a colour response. Actual taps always come from the
/// engine's offer in `sources`, including its timing and conditional checks.
fn remaining_sources(view: &PlayerView) -> Vec<Source> {
    let mut estimate = LegalActions::default();
    for object in view.battlefield_of(view.seat) {
        if object.status.contains(baylee_view::ObjectStatus::TAPPED)
            || object
                .status
                .contains(baylee_view::ObjectStatus::PHASED_OUT)
            || (object.summoning_sick
                && object.types.contains(TypeSet::CREATURE)
                && object.keywords & KeywordSet::HASTE.bits() == 0)
        {
            continue;
        }
        if manaplan::basic_land_color(&object.subtypes).is_some() {
            estimate.mana_abilities.push(object.id);
        }
        if let Some(grant) = &object.granted_mana {
            estimate.abilities.push((
                object.id,
                baylee_engine::choice::granted_ability(grant.slot),
            ));
        }
        // What the object can tap for, which for a copy is what the copied
        // card prints (CR 707.2) — the index is into that list, as the
        // engine's offer is.
        for (index, ability) in crate::activate::printed_list(object).iter().enumerate() {
            // Conditional activations need an offer to certify them. The
            // colour estimate can omit a source but must not rely on one.
            if matches!(
                ability,
                AbilityDef::Activated {
                    mana_ability: true,
                    ..
                }
            ) {
                estimate
                    .abilities
                    .push((object.id, u32::try_from(index).unwrap_or(u32::MAX)));
            }
        }
    }
    sources(view, &estimate)
}

#[cfg(test)]
mod tests {
    use super::{mix, priced};
    use baylee_cards_dsl::{Cost, CostPart};

    /// **A land on the back of a card is still a land the engine offers.**
    ///
    /// A `CardIdentity` in hand names the face that is up, and for a modal
    /// double-faced card that face is the spell — so every reader asking
    /// `types.contains(LAND)` of it read Shatterskull Smashing as a sorcery
    /// and nothing else, while `compute_legal` was already offering the land
    /// drop (CR 712.12). An agent that disagrees with the offer it is
    /// answering declines land drops the engine is making it.
    ///
    /// So the rule is read over the **whole pool** rather than over an
    /// example, and in two populations, because the back face is a land drop
    /// for one kind of double-faced card and not the other. A modal card's
    /// land back is (CR 712.12); a transforming card's is not, since in hand
    /// it has only its front face's characteristics (CR 712.8a) — Arguel's
    /// Blood Fast is an enchantment there, and its Temple is reached only by
    /// turning over. `castable_from_hand` is what tells the two apart, and
    /// `xtask validate` holds it against Scryfall's `layout`.
    ///
    /// Both populations carry a floor and a card pinned by name, so an
    /// answer that is "yes" for every back land, or "no" for every one, is
    /// red here rather than a quiet pass over half the pool.
    #[test]
    fn a_land_on_a_modal_back_is_playable_and_one_on_a_transforming_back_is_not() {
        use baylee_core::ids::PrintRef;
        use baylee_core::types::TypeSet;

        let identity = |index| super::CardIdentity {
            index,
            print: PrintRef::new(0),
            face: 0,
        };
        let by_oracle = |oracle_id| {
            identity(
                baylee_cards::by_oracle_id(oracle_id)
                    .expect("registry contains the card")
                    .index,
            )
        };
        let mut read = 0usize;
        let mut modal = Vec::new();
        let mut transforming = Vec::new();
        let mut wrong = Vec::new();
        for def in baylee_cards::all() {
            read += 1;
            // Face 0, because that is what a card in hand shows and what
            // every front-face reader was looking at.
            let card = identity(def.index);
            let front_is_a_land = def.faces[0].types.contains(TypeSet::LAND);
            let back_land = def
                .faces
                .iter()
                .skip(1)
                .find(|f| f.types.contains(TypeSet::LAND));
            let playable = match back_land {
                _ if front_is_a_land => true,
                Some(back) if back.castable_from_hand => {
                    modal.push(def.name());
                    true
                }
                Some(_) => {
                    transforming.push(def.name());
                    false
                }
                None => false,
            };
            if super::plays_as_land(card) != playable {
                wrong.push(def.name());
            }
            if playable {
                assert!(
                    super::land_face(card).is_some_and(|f| f.types.contains(TypeSet::LAND)),
                    "{} names a face that is not the land",
                    def.name()
                );
            }
        }

        assert!(
            read > 2_500,
            "read {read} cards out of the pool, which is not the pool"
        );
        assert!(
            wrong.is_empty(),
            "{} card(s) disagree with CR 712.12 / 712.8a: {:?}",
            wrong.len(),
            &wrong[..wrong.len().min(10)]
        );
        // Measured 2026-09-24: 50 modal and 32 transforming, over 2716
        // cards. Floors under both, because the pool only grows, and a
        // sweep that read none of either would report the same clean result
        // as one that read all of them.
        assert!(
            modal.len() >= 48,
            "only {} modal card(s) print a spell over a land: {modal:?}",
            modal.len()
        );
        assert!(
            transforming.len() >= 30,
            "only {} transforming card(s) turn into a land: {transforming:?}",
            transforming.len()
        );

        // Pinned by name, one of each kind.
        let shatterskull = by_oracle("78301998-fd9b-4cd5-afad-dbcb43cac2a7");
        assert!(super::plays_as_land(shatterskull), "Shatterskull Smashing");
        let blood_fast = by_oracle("be2a4bc4-8af6-48c5-9421-32d26272e71a");
        assert!(!super::plays_as_land(blood_fast), "Arguel's Blood Fast");
        assert!(super::land_face(blood_fast).is_none());

        // And a card with no land face anywhere is not a land drop.
        let bolt = by_oracle("4457ed35-7c10-48c8-9776-456485fdf070");
        assert!(!super::plays_as_land(bolt));
        assert!(super::land_face(bolt).is_none());
    }

    /// What a tap costs is part of the ranking. `mana_shape` has already
    /// refused a mana price, so what is left is `cost.parts`, and `{T}` is
    /// the only part a mana source may carry for free — a `matches!` on
    /// that one variant rather than a list of the expensive ones, so a
    /// `CostPart` added tomorrow is priced by default.
    ///
    /// The bug it closes is Havenwood Battleground, which prints `{T}: Add
    /// {G}` beside `{T}, Sacrifice this land: Add {G}{G}`: ranked on mana
    /// made, the sacrifice won and the agent sold the land for a mana it
    /// already had.
    #[test]
    fn only_tapping_the_permanent_is_free() {
        assert!(!priced(&Cost::TAP));
        assert!(!priced(&Cost::FREE), "an ability with no cost at all");
        assert!(priced(&baylee_cards_dsl::cost!(TapSelf, SacrificeSelf)));
        assert!(
            priced(&baylee_cards_dsl::cost!(TapSelf, PayLife(1))),
            "Spire of Industry pays the life whenever the plan asks it to"
        );
        assert!(
            priced(&baylee_cards_dsl::cost!(UntapSelf)),
            "untapping a permanent is a different button from tapping it, \
             whatever mana comes out"
        );
        assert!(priced(&baylee_cards_dsl::cost!(
            TapSelf,
            Discard(&baylee_cards_dsl::Filter::CREATURE)
        )));
    }

    /// The noise an agent breaks a tie with is `SplitMix`'s integer
    /// finalizer: fixed arithmetic, not a platform hasher and not a process
    /// RNG. A replay has to reach the same decision on another machine and
    /// in another build, so the values are written down rather than merely
    /// asserted to be stable within one run — a `DefaultHasher` passes
    /// every property test this could ask and changes these four numbers.
    #[test]
    fn the_tie_break_noise_is_arithmetic_and_not_a_hasher() {
        assert_eq!(mix(0), 0);
        assert_eq!(mix(1), 6_238_072_747_940_578_789);
        assert_eq!(mix(2), 15_839_785_061_582_574_730);
        assert_eq!(mix(0xdead_beef), 5_622_224_078_331_092_714);
    }

    /// And it separates neighbours, which is the whole job: the noise is
    /// keyed by the offered object as well as the view, so two objects one
    /// apart must not sort together.
    #[test]
    fn neighbouring_keys_do_not_share_a_tie_break() {
        let noise: Vec<u64> = (0..64).map(mix).collect();
        let mut sorted = noise.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), noise.len(), "64 keys, 64 distinct answers");
        assert_ne!(noise[1] >> 60, noise[2] >> 60, "and they differ up top");
    }

    /// One part of `priced`'s contract that the `matches!` states and
    /// nothing else would: a cost made only of parts it has never seen is
    /// priced, not free.
    #[test]
    fn an_unrecognised_part_is_priced_rather_than_ignored() {
        const PARTS: [&[CostPart]; 3] = [
            &[CostPart::ExileSelf],
            &[CostPart::DiscardSelf],
            &[CostPart::ReturnSelfToHand],
        ];
        for parts in PARTS {
            let cost = Cost {
                mana: baylee_core::mana::ManaCost::ZERO,
                parts,
            };
            assert!(priced(&cost), "{parts:?}");
        }
    }
}
