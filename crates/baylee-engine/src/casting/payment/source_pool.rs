//! Solve ordinary and imposing-source mana together, then debit exact receipts.
#[allow(clippy::wildcard_imports)]
use super::*;

pub(super) struct Payment<'a> {
    pub state: &'a GameState,
    pub real: &'a ManaPool,
    pub entries: ManaEntries,
    pub riding: ManaEntries,
    pub cost: &'a ManaCost,
    pub spending: ManaSpending,
    pub restriction: Option<crate::mana_pay::GenericRestriction>,
    pub allowance: SourceAllowance,
    pub obligation: Option<&'a crate::constrained_payment::ConstrainedPayment>,
    pub must_consume: bool,
}

impl Payment<'_> {
    pub(super) fn pay(self) -> Option<(ManaPool, ManaEntries, [u32; 6])> {
        let (credits, normal): (ManaEntries, ManaEntries) = self
            .entries
            .into_iter()
            .partition(|(mana, ..)| super::super::source_cost::is_source_credit(self.state, *mana));
        let ordinary = merged(self.real, &normal);
        let credit_pool = merged(&ManaPool::new(), &credits);
        let mut prefer = [0_u32; 6];
        for (mana, ..) in normal.iter().chain(
            self.riding
                .iter()
                .filter(|(_, _, rider)| *rider != SpendRider::None),
        ) {
            prefer[mana.color.index()] =
                prefer[mana.color.index()].saturating_add(u32::from(mana.amount));
        }
        let required = self.obligation.map(|payment| &payment.required);
        let (normal_required, credit_required) = required_classes(self.state, required)?;
        for (preference, &minimum) in prefer.iter_mut().zip(normal_required.iter()) {
            *preference = (*preference).max(minimum);
        }
        let paid = if credits.is_empty() && required.is_none() && self.restriction.is_none() {
            mana_pay::SourcePayment {
                restricted_generic_paid: [0; 6],
                remaining: mana_pay::payment_consuming(
                    &ordinary,
                    self.cost,
                    self.spending,
                    prefer,
                    self.restriction,
                    if self.must_consume {
                        normal_required
                    } else {
                        [0; 6]
                    },
                )?,
                restricted_remaining: credit_pool.clone(),
            }
        } else {
            mana_pay::payment_with_source_receipts(
                &ordinary,
                self.cost,
                self.spending,
                prefer,
                self.restriction,
                if self.must_consume {
                    normal_required
                } else {
                    [0; 6]
                },
                mana_pay::SourceCredits {
                    pool: credit_pool.clone(),
                    required: if self.must_consume {
                        credit_required
                    } else {
                        [0; 6]
                    },
                    generic_quota: self.allowance.quota(self.cost),
                    constrained: self.allowance.constrained,
                },
                if self.must_consume {
                    receipt_requirements(self.state, required)?
                } else {
                    [mana_pay::ReceiptRequirement::default(); 2]
                },
            )?
        };
        let (normal_plain, normal_snow) = consumed_class(&ordinary, &paid.remaining);
        let (credit_plain, credit_snow) = consumed_class(&credit_pool, &paid.restricted_remaining);
        let (pool, mut spent) =
            debit_tracked_units(self.real, normal, self.riding, normal_plain, normal_snow)?;
        let (pool, credit_spent) =
            debit_tracked_units(&pool, credits, SmallVec::new(), credit_plain, credit_snow)?;
        spent.extend(credit_spent);
        Some((pool, spent, paid.restricted_generic_paid))
    }
}

fn required_classes(
    state: &GameState,
    required: Option<&ManaPool>,
) -> Option<([u32; 6], [u32; 6])> {
    let Some(required) = required else {
        return Some(([0; 6], [0; 6]));
    };
    let mut ordinary = ManaColor::ALL.map(|color| required.available(color));
    let mut credit = [0_u32; 6];
    for mana in required.restricted() {
        let target = if super::super::source_cost::is_source_credit(state, *mana) {
            &mut credit
        } else {
            &mut ordinary
        };
        let slot = &mut target[mana.color.index()];
        *slot = slot.checked_add(u32::from(mana.amount))?;
    }
    Some((ordinary, credit))
}

fn consumed_class(before: &ManaPool, after: &ManaPool) -> ([u32; 6], [u32; 6]) {
    let snow =
        ManaColor::ALL.map(|color| before.snow_available(color) - after.snow_available(color));
    let plain = ManaColor::ALL
        .map(|color| before.available(color) - after.available(color) - snow[color.index()]);
    (plain, snow)
}

fn receipt_requirements(
    state: &GameState,
    required: Option<&ManaPool>,
) -> Option<[mana_pay::ReceiptRequirement; 2]> {
    let mut result = [mana_pay::ReceiptRequirement::default(); 2];
    let Some(required) = required else {
        return Some(result);
    };
    for color in ManaColor::ALL {
        result[0].snow[color.index()] = required.snow_available(color);
        result[0].plain[color.index()] = required.available(color) - required.snow_available(color);
    }
    for mana in required.restricted() {
        let class = usize::from(super::super::source_cost::is_source_credit(state, *mana));
        let counts = if mana.flags.contains(ManaFlags::SNOW) {
            &mut result[class].snow
        } else {
            &mut result[class].plain
        };
        counts[mana.color.index()] =
            counts[mana.color.index()].checked_add(u32::from(mana.amount))?;
    }
    Some(result)
}
