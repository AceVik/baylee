//! One-turn rules permissions: value the follow-up they can actually enable.

use baylee_cards_dsl::{
    AbilityDef, Amount, Cost, CostPart, Effect, FaceDef, KeywordSet, ManaSource, Modifier,
    PlayerRel,
};
use baylee_client_core::manaplan::{self, Source};
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::TypeSet;
use baylee_view::{
    CardIdentity, ManaPoolView, ObjectStatus, Phase, PlayerView, PublicObject, Step,
};

use super::{CARD, DECKED, Origin};
use crate::HeuristicAgent;

/// Only explicitly modelled permissions are readable; unknown modifiers still
/// return `None`. Setup mana and a consumed source cannot fund the follow-up.
pub(super) fn worth(
    agent: &HeuristicAgent,
    view: &PlayerView,
    origin: Origin<'_>,
    modifier: &Modifier,
    setup: &Cost,
) -> Option<i64> {
    match modifier {
        Modifier::NextDrawWish => Some(draw_wish(view)),
        Modifier::NextSpellUncounterable | Modifier::OneSpellManaAnyType => {
            let protection = *modifier == Modifier::NextSpellUncounterable;
            if protection && !counterspell_risk(agent, view) {
                return Some(0);
            }
            let budget = Budget::new(view, origin, setup);
            Some(
                view.hand
                    .iter()
                    .filter_map(|card| {
                        let face = cast_this_turn(view, card.card)?;
                        let score = agent.spell_score(view, card.card);
                        if score <= 0 {
                            return None;
                        }
                        if protection {
                            let uncounterable = baylee_cards::by_index(card.card.index)?
                                .keywords_for_face(usize::from(card.card.face))
                                .contains(KeywordSet::UNCOUNTERABLE);
                            (!uncounterable && budget.pays(&face.mana_cost, false))
                                .then_some((score / 3).clamp(80, CARD))
                        } else {
                            // Already-payable spells gain nothing; North Star
                            // changes the base cost's payment only, not costs
                            // of either activation or additional spell costs.
                            (!budget.pays(&face.mana_cost, false)
                                && budget.pays(&face.mana_cost, true))
                            .then_some((score / 2).clamp(80, CARD * 2))
                        }
                    })
                    .max()
                    .unwrap_or(0),
            )
        }
        Modifier::GraveyardVampireOnce => {
            let budget = Budget::new(view, origin, setup);
            Some(
                view.graveyards
                    .iter()
                    .flatten()
                    .filter(|card| {
                        card.owner == view.seat && card.types.contains(TypeSet::CREATURE)
                    })
                    .filter_map(|card| {
                        let identity = card.card?;
                        let face = cast_this_turn(view, identity)?;
                        if !budget.pays(&face.mana_cost, false) {
                            return None;
                        }
                        let score = agent.spell_score(view, identity);
                        // The card is recovered once, with a modest deduction
                        // for finality preventing its next graveyard reuse.
                        (score > 0).then_some((score / 2).clamp(80, CARD * 2) - 40)
                    })
                    .max()
                    .unwrap_or(0),
            )
        }
        _ => None,
    }
}

/// This is an estimate from public sources, exactly as `policy::can_pay` is.
/// The legal offer remains authoritative about restrictions and cost changes.
struct Budget {
    pool: ManaPoolView,
    sources: Vec<Source>,
    setup: ManaCost,
}

impl Budget {
    fn new(view: &PlayerView, origin: Origin<'_>, setup: &Cost) -> Self {
        let mut sources = crate::policy::remaining_sources(view);
        if setup.parts.iter().any(|part| {
            matches!(
                part,
                CostPart::TapSelf | CostPart::SacrificeSelf | CostPart::ExileSelf
            )
        }) {
            sources.retain(|source| source.id != origin.id);
        }
        Self {
            pool: view
                .seat(view.seat)
                .map_or_else(ManaPoolView::default, |seat| seat.mana_pool),
            sources,
            setup: setup.mana,
        }
    }

    fn pays(&self, printed: &ManaCost, any_type: bool) -> bool {
        // Never assume that the player will pay life for Phyrexian symbols.
        let base = printed.with_phyrexian_settled(0);
        let total = self.setup.combine(&base);
        let payment = if any_type {
            total.with_any_type_component(&base)
        } else {
            total
        };
        manaplan::plan(&payment, &self.pool, &self.sources).is_some()
    }
}

/// No speculative future turn, free cast of a missing cost, or unpriced
/// mandatory additional cost. Variable spells need a separate X plan.
fn cast_this_turn(view: &PlayerView, card: CardIdentity) -> Option<&'static FaceDef> {
    let face = crate::policy::face(card)?;
    if !face.castable_from_hand
        || face.types.contains(TypeSet::LAND)
        || face.mana_cost.is_empty()
        || face.mana_cost.has_variable()
        || !face.mandatory_additional_costs.is_empty()
    {
        return None;
    }
    let main = view.active == view.seat
        && view.stack.is_empty()
        && matches!(view.phase, Phase::FirstMain | Phase::SecondMain);
    let fast = face.types.contains(TypeSet::INSTANT)
        || face.types.contains(TypeSet::SORCERY) && view.sorceries_have_flash
        || baylee_cards::by_index(card.index)?
            .keywords_for_face(usize::from(card.face))
            .contains(KeywordSet::FLASH);
    (main || fast && view.sorcery_lock.is_none()).then_some(face)
}

/// Blue mana and an opposing hand suggest counterplay without claiming to
/// know its hidden contents. A tapped-out opponent offers no such evidence.
fn counterspell_risk(agent: &HeuristicAgent, view: &PlayerView) -> bool {
    view.seats.iter().any(|seat| {
        agent.hostile(seat.player, view.seat)
            && !seat.has_lost()
            && seat.hand_count > 0
            && (seat.mana_pool.blue > 0 || view.battlefield_of(seat.player).any(blue_source))
    })
}

fn blue_source(object: &PublicObject) -> bool {
    if object.status.contains(ObjectStatus::TAPPED)
        || object.status.contains(ObjectStatus::PHASED_OUT)
        || object.summoning_sick
            && object.types.contains(TypeSet::CREATURE)
            && !super::has(object, KeywordSet::HASTE)
    {
        return false;
    }
    if manaplan::basic_land_color(&object.subtypes) == Some(ManaColor::Blue)
        || object
            .granted_mana_options()
            .any(|mana| mana.amount > 0 && mana.colors.contains(&ManaColor::Blue))
    {
        return true;
    }
    crate::activate::printed_list(object).iter().any(|ability| {
        let AbilityDef::Activated {
            cost,
            effects,
            mana_ability: true,
            ..
        } = ability
        else {
            return false;
        };
        if !cost.mana.is_empty() || cost.parts.iter().any(|part| *part != CostPart::TapSelf) {
            return false;
        }
        match baylee_cards_dsl::mana_with_riders(cost, effects) {
            Some((ManaSource::Fixed(ManaColor::Blue), Some(amount), _)) => amount > 0,
            Some((ManaSource::Choice(colors), Some(amount), _)) => {
                amount > 0 && colors.contains(&ManaColor::Blue)
            }
            _ => false,
        }
    })
}

/// A wish replaces, rather than adds, a draw. No sideboard inventory is sent
/// in the view, so only a proven prevention of decking earns a benefit.
fn draw_wish(view: &PlayerView) -> i64 {
    let natural_draw = view.active == view.seat && matches!(view.step, Step::Untap | Step::Upkeep);
    // Read certain, untargeted draws only: a spell aimed at an opponent or a
    // conditional branch must not pretend that this seat will draw instead.
    let pending_draws = view
        .stack
        .iter()
        .filter(|spell| spell.controller == view.seat)
        .flat_map(crate::tactics::printed_effects)
        .flatten()
        .filter_map(|effect| match effect {
            Effect::DrawCards {
                amount: Amount::Fixed(n),
            }
            | Effect::DrawThenDiscardDrawn {
                draw: Amount::Fixed(n),
                ..
            }
            | Effect::DrawCardsFor {
                amount: Amount::Fixed(n),
                who: PlayerRel::You,
            } => Some(*n),
            _ => None,
        })
        .fold(0_u32, u32::saturating_add);
    let draws = pending_draws.saturating_add(u32::from(natural_draw));
    if draws == 0 {
        return 0;
    }
    let library = view.seat(view.seat).map_or(0, |seat| seat.library_count);
    if draws > library.saturating_add(1) {
        0
    } else if draws > library {
        // The replacement prevents this draw even when no outside card can
        // be chosen. It protects one draw, not an entire multi-card effect.
        DECKED
    } else {
        0
    }
}
