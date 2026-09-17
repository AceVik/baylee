//! Decisions about the hand and mana use only printed cards and the seat's
//! own view. In particular, an unseen card never acquires a registry identity.

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
    pub(crate) fn noise(&self, view: &PlayerView, id: ObjectId) -> i64 {
        let width = u64::from(self.profile.temperature_milli.min(10_000));
        if width == 0 {
            return 0;
        }
        let seed = view.seq ^ (u64::from(id.slot()) << 16) ^ u64::from(view.seat.get());
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
        let lands = view
            .hand
            .iter()
            .filter(|c| c.types.contains(TypeSet::LAND))
            .count();
        let balance = (2..=5).contains(&lands);
        let curve = skill < 2
            || view.hand.iter().any(|card| {
                !card.types.contains(TypeSet::LAND)
                    && card.mana_value <= 3
                    && face(card.card).is_some_and(|f| {
                        let colors = view
                            .hand
                            .iter()
                            .filter(|c| c.types.contains(TypeSet::LAND))
                            .filter_map(|c| face(c.card))
                            .fold(baylee_core::color::ColorSet::EMPTY, |colors, land| {
                                let mut subtypes = baylee_core::types::SubtypeSet::EMPTY;
                                for &subtype in land.subtypes {
                                    subtypes.insert(subtype);
                                }
                                let color = manaplan::basic_land_color(&subtypes);
                                color.map_or(colors, |color| colors.union(color_set(color)))
                            });
                        // Nonbasic colour sources may need board context. Do not
                        // reject an otherwise playable hand on an unknown colour.
                        f.mana_cost.colors().is_empty()
                            || f.mana_cost.colors().intersects(colors)
                            || view.hand.iter().any(|c| {
                                c.types.contains(TypeSet::LAND)
                                    && face(c.card).is_some_and(|land| land.subtypes.is_empty())
                            })
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
        let mut lands = hand
            .iter()
            .filter(|c| c.types.contains(TypeSet::LAND))
            .count();
        let mut result = Vec::with_capacity(count);
        for _ in 0..count.min(hand.len()) {
            let worst = hand
                .iter()
                .enumerate()
                .min_by_key(|(_, c)| {
                    let value = if c.types.contains(TypeSet::LAND) {
                        if lands > 3 { -1000 } else { 2000 }
                    } else {
                        700 - i64::from(c.mana_value) * 100
                    };
                    (value, c.id)
                })
                .map_or(0, |(i, _)| i);
            let card = hand.remove(worst);
            if card.types.contains(TypeSet::LAND) {
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
            let cost = spell_cost(view, id, f);
            let action = if legal.castable.contains(&id) {
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
        let enemy_stack = view
            .stack
            .iter()
            .any(|o| self.hostile(o.controller, view.seat));
        let enemy_board = view
            .battlefield
            .iter()
            .any(|o| self.hostile(o.controller, view.seat) && !o.types.contains(TypeSet::LAND));
        let mut value = 200 + i64::from(f.mana_cost.cmc()) * 100;
        if f.types.contains(TypeSet::CREATURE) {
            value += 250 + i64::from(f.power.unwrap_or(0)) * 60;
        }
        for effect in effects.flatten() {
            // A deferred loss is not a free counter. The current view has no
            // future payment window to plan against; decline an obligation
            // this stateless policy cannot discharge.
            if matches!(effect, Effect::PayCostOrLoseLater { .. }) {
                return -10_000;
            }
            if counter(effect) && !enemy_stack {
                return -10_000;
            }
            if removal(effect) && !enemy_board {
                return -10_000;
            }
            if counter(effect) {
                value += 1500;
            }
            if removal(effect) {
                value += 400;
            }
            if matches!(
                effect,
                Effect::DrawCards { .. } | Effect::LookAtTopPick { .. }
            ) {
                value += 300;
            }
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

/// Read only offered, simple mana taps. One permanent is one source even
/// when its intrinsic and printed abilities both appear in the offer.
fn sources(view: &PlayerView, legal: &LegalActions) -> Vec<Source> {
    let mut result = Vec::new();
    for &id in &legal.mana_abilities {
        if let Some(color) = view
            .object(id)
            .and_then(|o| manaplan::basic_land_color(&o.subtypes))
        {
            result.push(Source::fixed(id, Tap::Intrinsic, color));
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
                .map(|m| (m.colors.clone(), m.amount))
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
                Some((colors, amount?))
            })
        };
        if let Some((colors, amount)) = source {
            result.push(Source {
                id,
                tap: Tap::Ability(index),
                colors,
                amount,
            });
        }
    }
    result.sort_by_key(|s| {
        (
            s.id,
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
