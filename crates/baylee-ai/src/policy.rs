//! Hand and mana decisions use legal offers and printed properties. A private
//! scouting summary may adjust values without supplying actionable hidden ids.

use baylee_cards_dsl::{AbilityDef, Effect, FaceDef, KeywordSet, ManaSource};
use baylee_client_core::manaplan::{self, Source, Tap};
use baylee_core::ids::ObjectId;
use baylee_core::mana::ManaColor;
use baylee_core::preset::HoldUp;
use baylee_core::types::TypeSet;
use baylee_engine::choice::{LegalActions, PlayerAction};
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
/// does not — `compute_legal` offers the land drop when **any** face is a
/// land (CR 712.12) — so the agent was refusing to count a card the engine
/// was already offering it. 82 cards in this pool print a spell over a land.
///
/// The rule here is deliberately the engine's rule and not a better one. It
/// does not ask whether the back is reached by *playing* it or by
/// transforming (CR 712.2); if that distinction is wrong it is wrong in
/// `compute_legal` first, and an agent that disagreed with the offer it is
/// answering would decline land drops the engine is making it.
pub(crate) fn land_face(card: CardIdentity) -> Option<&'static FaceDef> {
    baylee_cards::by_index(card.index)?
        .faces
        .iter()
        .find(|f| f.types.contains(TypeSet::LAND))
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
        if self.profile.mulligan_skill < 2 {
            return None;
        }
        let lands = view
            .battlefield_of(view.seat)
            .filter(|o| o.types.contains(TypeSet::LAND))
            .count();
        let value = |id: &ObjectId| -> i64 {
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
        };
        let mut ranked = options.to_vec();
        let count = match prompt {
            ChoicePrompt::SearchLibrary | ChoicePrompt::Wish => {
                ranked.sort_by_key(|id| (std::cmp::Reverse(value(id)), *id));
                usize::from(max)
            }
            ChoicePrompt::ScryBottom | ChoicePrompt::SurveilGraveyard => {
                // Unknown cards are kept. An id alone is not information
                // about the top of a library.
                ranked.sort_by_key(|id| (value(id), *id));
                ranked
                    .iter()
                    .take_while(|id| value(id) < 0)
                    .count()
                    .clamp(usize::from(min), usize::from(max))
            }
            ChoicePrompt::CostSacrifice
            | ChoicePrompt::CostDiscard
            | ChoicePrompt::PutBackOnTop => {
                ranked.sort_by_key(|id| (value(id), *id));
                usize::from(min)
            }
            _ => return None,
        };
        ranked.truncate(count);
        Some(ranked)
    }

    pub(crate) fn color(&self, view: &PlayerView, options: &[ManaColor]) -> ManaColor {
        if self.profile.mulligan_skill >= 2
            && let Some(color) = self.planned_color(view, options)
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
                    .map(|c| c.card)
                    .chain(
                        view.command
                            .get(usize::from(view.seat.get()))
                            .into_iter()
                            .flatten()
                            .filter(|o| o.commander)
                            .filter_map(|o| o.card),
                    )
                    .filter_map(face)
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
    fn planned_color(&self, view: &PlayerView, options: &[ManaColor]) -> Option<ManaColor> {
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
        let sources = sources(view, legal);
        let main = view.active == view.seat
            && view.stack.is_empty()
            && matches!(
                view.phase,
                baylee_view::Phase::FirstMain | baylee_view::Phase::SecondMain
            );
        let available = pool.total() + sources.iter().map(|s| u32::from(s.amount)).sum::<u32>();
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
            let base_cost = spell_cost(view, id, f);
            let variable = base_cost.has_variable();
            let cost = if variable {
                (1..=available.min(50))
                    .rev()
                    .find_map(|x| {
                        let cost = base_cost.with_x(x);
                        manaplan::plan(&cost, pool, &sources)
                            .is_some()
                            .then_some(cost)
                    })
                    .unwrap_or_else(|| base_cost.with_x(0))
            } else {
                base_cost
            };
            let action = if legal.castable.contains(&id)
                && (!variable || manaplan::plan(&cost, pool, &[]).is_some())
            {
                PlayerAction::CastSpell { card: id }
            } else {
                let Some(plan) = manaplan::plan(&cost, pool, &sources) else {
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
            if main && !instant && available.saturating_sub(cost.cmc()) < reserve {
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
fn sources(view: &PlayerView, legal: &LegalActions) -> Vec<Source> {
    // Paired with its price until the dedup has run, rather than carried on
    // `manaplan::Source`: the price is what goes *in*, and the solver only
    // ever asks what comes out. `baylee-client-7e` reached the same split
    // from the client's side in #165.
    //
    // **`bundle` on `Source` is not this flag** and the two will sit three
    // lines apart once #150 lands. `priced` is what the tap *costs*; `bundle`
    // is how `colors` is *read* — one of each, or a choice of one. Folding
    // them, or moving `bundle` into this tuple, compiles and passes every
    // test on either side, because the client's tests build a `Source`
    // directly and never come through here. Three readers currently agree
    // about the same family of lands from three directions — `mana_bundle`,
    // `duplicates_intrinsic` and this column — and one flag doing two jobs
    // would silently make that one reader wearing three names.
    let mut result: Vec<(Source, bool)> = Vec::new();
    for &id in &legal.mana_abilities {
        if let Some(color) = view
            .object(id)
            .and_then(|o| manaplan::basic_land_color(&o.subtypes))
        {
            // CR 305.6: the intrinsic tap of a basic land type costs the tap
            // and nothing else.
            result.push((Source::fixed(id, Tap::Intrinsic, color), false));
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
                .map(|m| (m.colors.clone(), m.amount, false))
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
                let (kind, amount, restricted) = baylee_cards_dsl::mana_shape(cost, effects)?;
                if restricted {
                    return None;
                }
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
                Some((colors, amount?, priced(cost)))
            })
        };
        if let Some((colors, amount, priced)) = source {
            result.push((
                Source {
                    id,
                    tap: Tap::Ability(index),
                    colors,
                    amount,
                },
                priced,
            ));
        }
    }
    result.sort_by_key(|(s, priced)| {
        (
            s.id,
            *priced,
            std::cmp::Reverse(s.amount),
            std::cmp::Reverse(s.colors.len()),
            matches!(s.tap, Tap::Ability(_)),
        )
    });
    result.dedup_by_key(|(s, _)| s.id);
    result.into_iter().map(|(source, _)| source).collect()
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
        if let Some(card) = object.card
            && let Some(def) = baylee_cards::by_index(card.index)
        {
            for (index, ability) in def
                .abilities_for_face(usize::from(card.face))
                .iter()
                .enumerate()
            {
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
    }
    sources(view, &estimate)
}
