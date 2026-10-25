//! Permissions belong to the source imposing each part of a total cost.
//! Keep this small value with a locked price across mana-ability windows.

use super::{GameState, ManaCost, ObjectId, PlayerId, SpendFor, payment::spending_object};
use super::{ManaPool, ManaSpending, RestrictedMana, payment::ManaEntries};

/// Generic reductions may discharge either component of a mixed total. We
/// retain the largest legal eligible remainder, rather than assigning a
/// reduction to an eligible component and refusing a possible payment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) struct SourceAllowance {
    /// All constrained symbols come from the base cost in the current DSL.
    pub constrained: bool,
    /// Eligible increase for an ordinary base, ordinary floor for an Un base.
    pub generic: u32,
}

impl SourceAllowance {
    pub(crate) const fn base(constrained: bool) -> Self {
        Self {
            constrained,
            generic: 0,
        }
    }

    pub(crate) fn adjusted(self, eligible: u32, ordinary: u32, reduction: u32) -> Self {
        Self {
            constrained: self.constrained,
            generic: if self.constrained {
                self.generic
                    .saturating_add(ordinary)
                    .saturating_sub(reduction)
            } else {
                self.generic.saturating_add(eligible)
            },
        }
    }

    pub(crate) fn reduced(self, amount: u32) -> Self {
        self.adjusted(0, 0, amount)
    }

    pub(crate) fn quota(self, cost: &ManaCost) -> u32 {
        if self.constrained {
            cost.generic_total().saturating_sub(self.generic)
        } else {
            self.generic.min(cost.generic_total())
        }
    }
}

pub(crate) fn base_allowance(state: &GameState, what: SpendFor) -> SourceAllowance {
    SourceAllowance::base(spending_object(state, what).is_some_and(|source| source.is_un_card()))
}

/// Read the imposing card's physical rule facts while determining the price.
/// The returned allowance, not a later source lookup, accompanies a locked cost.
pub(crate) fn effect_is_un(state: &GameState, source: Option<ObjectId>) -> bool {
    source
        .and_then(|source| state.object(source))
        .is_some_and(crate::object::GameObject::is_un_card)
}

pub(crate) fn live_allowance(
    state: &GameState,
    player: PlayerId,
    what: SpendFor,
) -> SourceAllowance {
    let base = base_allowance(state, what);
    let Some(object) = spending_object(state, what) else {
        return base;
    };
    match what {
        SpendFor::Other | SpendFor::OtherFrom(_) | SpendFor::CumulativeUpkeep(_) => base,
        SpendFor::Ability(source) => {
            let (eligible, ordinary) = activation_increases(state, source);
            base.adjusted(eligible, ordinary, 0)
        }
        _ => {
            use crate::state::CardLookup;
            let printed = if matches!(
                what,
                SpendFor::SpellAs(_, super::SpellForm::Disguise)
                    | SpendFor::KickedSpell(_, _, Some(super::SpellForm::Disguise))
            ) {
                0
            } else {
                object
                    .card
                    .and_then(|card| state.bases.card(card.index))
                    .and_then(|def| def.faces.get(usize::from(object.face_index)))
                    .map_or(0, |face| {
                        super::printed_reduction(state, face, player, object.id)
                    })
            };
            let adjustment = super::spell_adjustment(state, player, &object);
            base.adjusted(
                adjustment.eligible_increase,
                adjustment
                    .increase
                    .saturating_sub(adjustment.eligible_increase),
                adjustment.reduction.saturating_add(printed),
            )
        }
    }
}

pub(crate) fn activation_increases(state: &GameState, source: ObjectId) -> (u32, u32) {
    if !state.effects.may_change_or_forbid_abilities() {
        return (0, 0);
    }
    let Some(object) = state.object(source) else {
        return (0, 0);
    };
    state
        .effects
        .iter()
        .fold((0_u32, 0_u32), |(eligible, ordinary), effect| {
            if let baylee_cards_dsl::Modifier::AbilitiesCostMore(amount) = effect.modifier
                && crate::effects::applies_to(state, effect, object)
            {
                if effect_is_un(state, effect.source) {
                    return (eligible.saturating_add(amount), ordinary);
                }
                return (eligible, ordinary.saturating_add(amount));
            }
            (eligible, ordinary)
        })
}

pub(super) fn has_source_credits(state: &GameState, pool: &ManaPool) -> bool {
    pool.restricted()
        .iter()
        .any(|mana| is_source_credit(state, *mana))
}

pub(super) fn is_source_credit(state: &GameState, mana: RestrictedMana) -> bool {
    state
        .restriction_info
        .get(&mana.restriction.0)
        .is_some_and(|provenance| {
            provenance.restriction.uses == baylee_cards_dsl::ManaUse::CostsFromMatchingSource
                && *provenance.restriction.filter == baylee_cards_dsl::Filter::UnCard
        })
}

pub(super) fn source_entries(state: &GameState, pool: &ManaPool) -> ManaEntries {
    pool.restricted()
        .iter()
        .filter_map(|mana| {
            if !is_source_credit(state, *mana) {
                return None;
            }
            let provenance = state.restriction_info.get(&mana.restriction.0)?;
            Some((
                *mana,
                provenance.context.source,
                provenance.restriction.rider,
            ))
        })
        .collect()
}

/// Planning pools contain ordinary admitted mana and retain all restriction
/// entries. Source credits were never added to the ordinary counters.
pub(super) fn separate_planning_pool(state: &GameState, pool: &ManaPool) -> (ManaPool, ManaPool) {
    let credits = super::merged(&ManaPool::new(), &source_entries(state, pool));
    (pool.clone(), credits)
}

/// The estimate and final payer use the same two-class symbol assignment.
#[allow(clippy::too_many_arguments)] // exact payment inputs, without hidden mutable context
pub(crate) fn source_plan(
    state: &GameState,
    _what: SpendFor,
    pool: &ManaPool,
    cost: &ManaCost,
    spending: ManaSpending,
    restriction: Option<crate::mana_pay::GenericRestriction>,
    allowance: SourceAllowance,
) -> Option<crate::mana_pay::SourcePayment> {
    if !has_source_credits(state, pool) {
        return crate::mana_pay::payment_restricting_generic(
            pool,
            cost,
            spending,
            [0; 6],
            restriction,
        )
        .map(|remaining| crate::mana_pay::SourcePayment {
            restricted_generic_paid: [0; 6],
            remaining,
            restricted_remaining: ManaPool::new(),
        });
    }
    let (ordinary, credits) = separate_planning_pool(state, pool);
    crate::mana_pay::payment_with_source_credits(
        &ordinary,
        cost,
        spending,
        [0; 6],
        restriction,
        [0; 6],
        crate::mana_pay::SourceCredits {
            pool: credits,
            required: [0; 6],
            generic_quota: allowance.quota(cost),
            constrained: allowance.constrained,
        },
    )
}

#[cfg(test)]
mod tests;
