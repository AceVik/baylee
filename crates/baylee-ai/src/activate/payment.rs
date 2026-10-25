//! Check the payable minimum of an X activation before announcing it.
use super::{AbilityDef, CostPart, LegalActions, ObjectId, PlayerView};
use baylee_cards_dsl::ActivationX;
use baylee_client_core::decision::resource_player;
use baylee_client_core::manaplan;

#[cfg(test)]
mod tests;

/// X offers deliberately precede making mana. Their fixed colored part is
/// not guaranteed payable by the offer: passing a short window reverses the
/// activation and restores exactly the offer a stateless house would retry.
/// Ordinary paid offers retain the engine's affordability guarantee.
pub(super) fn reachable(
    view: &PlayerView,
    legal: &LegalActions,
    source: ObjectId,
    definition: &AbilityDef,
) -> bool {
    let (AbilityDef::Activated {
        cost,
        x_value,
        cost_reduction: reduction,
        ..
    }
    | AbilityDef::ActivatedConditional {
        cost,
        x_value,
        cost_reduction: reduction,
        ..
    }) = definition
    else {
        return true;
    };
    if !cost.mana.has_variable() || reduction.is_some() {
        return true;
    }
    let minimum = match x_value {
        ActivationX::Any => 0,
        ActivationX::AtLeast(n) => *n,
        // A card-defined amount/reduction needs its own public evaluator;
        // do not reject an offer using a guessed effective cost.
        ActivationX::Defined(_) => return true,
    };
    let Some(seat) = view.seat(resource_player(view)) else {
        return false;
    };
    let mut sources = crate::policy::sources(view, legal);
    if cost.parts.contains(&CostPart::TapSelf) {
        sources.retain(|candidate| candidate.id != source);
    }
    let spends_life = cost.parts.iter().any(|part| {
        matches!(
            part,
            CostPart::PayLife(_) | CostPart::PayLifeX | CostPart::PayCommanderColors
        )
    });
    let cost = cost
        .mana
        .with_x(minimum)
        .with_more_generic(legal.activation_increase(source));
    let spending = view
        .object(source)
        .map_or(seat.mana_pool.spending, |o| o.ability_mana_spending);
    let mut pool = seat.mana_pool;
    pool.spending = spending;
    let affordable = |pool: &baylee_view::ManaPoolView| {
        let reachable = pool
            .total()
            .saturating_add(sources.iter().map(|s| u64::from(s.amount)).sum());
        u64::from(cost.cmc()) <= reachable && manaplan::plan(&cost, pool, &sources).is_some()
    };
    affordable(&pool)
        || (!spends_life
            && crate::granted::planned_pool(view, legal).is_some_and(|(_, mut pool)| {
                pool.spending = spending;
                affordable(&pool)
            }))
}
