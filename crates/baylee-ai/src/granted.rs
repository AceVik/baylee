//! Deliberate special-action uses from public offers, never speculative life payment.
use baylee_cards_dsl::{SpecialActionCost, SpecialActionTiming};
use baylee_client_core::manaplan::{self, Source, Tap};
use baylee_core::ids::{PlayerId, TargetRef};
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_engine::choice::{GrantedActionKind, LegalActions, PlayerAction};
use baylee_view::{ManaPoolView, PlayerView};

/// Complete an already chosen fixed payment, leaving at least one life.
/// Ordinary mana sources are used before making the explicit life payment.
pub(crate) fn pay_fixed(
    view: &PlayerView,
    legal: &LegalActions,
    cost: &ManaCost,
    sources: &[Source],
) -> Option<PlayerAction> {
    let seat = view.seat(baylee_client_core::decision::resource_player(view))?;
    let (grant, pool) = planned_pool(view, legal)?;
    let plan = manaplan::plan(cost, &pool, sources)?;
    if let Some(step) = plan.steps.first() {
        return Some(match step.tap {
            Tap::Intrinsic => PlayerAction::ActivateManaAbility {
                source: step.source,
            },
            Tap::Ability(ability_index) => PlayerAction::ActivateAbility {
                source: step.source,
                ability_index,
            },
        });
    }
    if manaplan::plan(cost, &seat.mana_pool, &[]).is_some() {
        return None;
    }
    Some(PlayerAction::TakeGrantedAction { id: grant })
}

pub(crate) fn planned_pool(
    view: &PlayerView,
    legal: &LegalActions,
) -> Option<(baylee_core::ids::GrantedActionId, ManaPoolView)> {
    let seat = view.seat(baylee_client_core::decision::resource_player(view))?;
    let life = u32::try_from(seat.life.saturating_sub(1)).unwrap_or(0);
    legal
        .granted_actions
        .iter()
        .filter_map(|grant| {
            let (
                SpecialActionCost::Life(cost),
                GrantedActionKind::AddMana {
                    color: ManaColor::Colorless,
                    amount,
                },
            ) = (grant.cost, grant.effect)
            else {
                return None;
            };
            if cost == 0 || cost > life || grant.timing != SpecialActionTiming::ManaAbility {
                return None;
            }
            let mut pool = seat.mana_pool;
            let extra = (life / cost)
                .saturating_mul(u32::from(amount))
                .min(u32::MAX - pool.colorless);
            pool.colorless += extra;
            Some((grant.id, pool))
        })
        .max_by_key(|(_, pool)| pool.colorless)
}

/// With no shield inventory in the public view, use only a single spare
/// floating unit against visible incoming damage. Do not tap out repeatedly
/// or spend life on speculative prevention.
pub(crate) fn protect(
    view: &PlayerView,
    legal: &LegalActions,
    hostile: impl Fn(PlayerId) -> bool,
) -> Option<PlayerAction> {
    let pool = &view
        .seat(baylee_client_core::decision::resource_player(view))?
        .mana_pool;
    if pool.total().saturating_sub(pool.restricted_total()) != 1 {
        return None;
    }
    legal.granted_actions.iter().find_map(|grant| {
        let (
            SpecialActionCost::Mana(cost),
            GrantedActionKind::PreventNextDamage { target, amount },
        ) = (grant.cost, grant.effect)
        else {
            return None;
        };
        if amount == 0 || cost.cmc() != 1 || manaplan::plan(&cost, pool, &[]).is_none() {
            return None;
        }
        let friendly = match target {
            TargetRef::Player(player) => !hostile(player),
            TargetRef::Object(reference) => view
                .target_object(reference)
                .is_some_and(|object| object.is_current && !hostile(object.controller)),
        };
        if !friendly
            || !view.stack.iter().any(|spell| {
                spell.targets.contains(&target)
                    && crate::tactics::stack_meaning(spell, 0).damage > 0
            })
        {
            return None;
        }
        Some(PlayerAction::TakeGrantedAction { id: grant.id })
    })
}

/// Reject casts whose optional mana plan requires lethal life expenditure.
pub(crate) fn viable_cast(
    view: &PlayerView,
    legal: &LegalActions,
    cost: &ManaCost,
    sources: &[Source],
) -> bool {
    if !legal
        .granted_actions
        .iter()
        .any(|grant| matches!(grant.effect, GrantedActionKind::AddMana { .. }))
    {
        return true;
    }
    let Some(seat) = view.seat(baylee_client_core::decision::resource_player(view)) else {
        return false;
    };
    manaplan::plan(cost, &seat.mana_pool, sources).is_some()
        || planned_pool(view, legal)
            .is_some_and(|(_, future)| manaplan::plan(cost, &future, sources).is_some())
}
