//! Joint assignment for costs with distinct imposing-source permissions.
//! Restricted units retain actual color and snow status through every branch.
use super::{
    ManaColor, ManaCost, ManaPool, ManaSpending, ManaSymbol, generic, preferred_first, spent,
};
use smallvec::{SmallVec, smallvec};

#[derive(Clone, Debug)]
pub(crate) struct SourceCredits {
    pub pool: ManaPool,
    pub required: [u32; 6],
    pub generic_quota: u32,
    pub constrained: bool,
}

pub(crate) struct SourcePayment {
    pub remaining: ManaPool,
    pub restricted_remaining: ManaPool,
    pub restricted_generic_paid: [u32; 6],
}

/// Exact obligated snow/plain subsets, in addition to per-type unit minima.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ReceiptRequirement {
    pub snow: [u32; 6],
    pub plain: [u32; 6],
}

struct Assignment {
    symbols: SmallVec<[ManaSymbol; 16]>,
    minimum: SmallVec<[u64; 17]>,
    original: [[u32; 6]; 2],
    original_pools: [ManaPool; 2],
    receipts: [ReceiptRequirement; 2],
    required: [[u32; 6]; 2],
    spending: ManaSpending,
    quota: u32,
    constrained: bool,
    restriction: Option<crate::mana_pay::GenericRestriction>,
}

pub(crate) fn payment_with_source_credits(
    pool: &ManaPool,
    cost: &ManaCost,
    spending: ManaSpending,
    prefer: [u32; 6],
    restriction: Option<crate::mana_pay::GenericRestriction>,
    required: [u32; 6],
    credits: SourceCredits,
) -> Option<SourcePayment> {
    payment_with_source_receipts(
        pool,
        cost,
        spending,
        prefer,
        restriction,
        required,
        credits,
        [ReceiptRequirement::default(); 2],
    )
}

#[allow(clippy::too_many_arguments)] // all resource constraints belong to one atomic assignment
pub(crate) fn payment_with_source_receipts(
    pool: &ManaPool,
    cost: &ManaCost,
    spending: ManaSpending,
    prefer: [u32; 6],
    restriction: Option<crate::mana_pay::GenericRestriction>,
    required: [u32; 6],
    credits: SourceCredits,
    receipts: [ReceiptRequirement; 2],
) -> Option<SourcePayment> {
    let original = [amounts(pool), amounts(&credits.pool)];
    let required = [required, credits.required];
    if (0..2).any(|class| (0..6).any(|color| required[class][color] > original[class][color])) {
        return None;
    }
    let mut symbols: SmallVec<[ManaSymbol; 16]> = cost.symbols().collect();
    symbols.sort_by_key(|symbol| match symbol {
        ManaSymbol::Hybrid(_) | ManaSymbol::HybridPhyrexian(_) | ManaSymbol::TwoOrColor(_) => {
            (1, 0)
        }
        ManaSymbol::Generic(_) => (2, 0),
        ManaSymbol::Colorless | ManaSymbol::Snow => (0, 0),
        _ => (0, 1),
    });
    let mut minimum: SmallVec<[u64; 17]> = smallvec![0; symbols.len() + 1];
    for (i, symbol) in symbols.iter().enumerate().rev() {
        minimum[i] = minimum[i + 1]
            + match symbol {
                ManaSymbol::Generic(amount) => u64::from(*amount),
                ManaSymbol::Variable(_) | ManaSymbol::HalfGeneric | ManaSymbol::Infinite => 0,
                _ => 1,
            };
    }
    Assignment {
        symbols,
        minimum,
        original,
        original_pools: [pool.clone(), credits.pool.clone()],
        receipts,
        required,
        spending,
        quota: credits.generic_quota,
        constrained: credits.constrained,
        restriction,
    }
    .assign([pool.clone(), credits.pool], 0, 0, 0, prefer, [[0; 6]; 2])
}

fn amounts(pool: &ManaPool) -> [u32; 6] {
    ManaColor::ALL.map(|color| pool.available(color))
}

impl Assignment {
    fn assign(
        &self,
        pools: [ManaPool; 2],
        index: usize,
        generic: u32,
        extra_quota: u32,
        prefer: [u32; 6],
        hard_snow: [[u32; 6]; 2],
    ) -> Option<SourcePayment> {
        let available: u64 = pools.iter().flat_map(amounts).map(u64::from).sum();
        if self.minimum[index] + u64::from(generic) > available {
            return None;
        }
        let Some(&symbol) = self.symbols.get(index) else {
            return self.generic(pools, generic, extra_quota, prefer, hard_snow);
        };
        match symbol {
            ManaSymbol::Generic(amount) => {
                return self.assign(
                    pools,
                    index + 1,
                    generic.checked_add(amount)?,
                    extra_quota,
                    prefer,
                    hard_snow,
                );
            }
            ManaSymbol::Variable(_) | ManaSymbol::HalfGeneric | ManaSymbol::Infinite => {
                return self.assign(pools, index + 1, generic, extra_quota, prefer, hard_snow);
            }
            _ => {}
        }
        for color in preferred_first(&self.options(symbol), prefer) {
            for class in [1, 0] {
                if class == 1 && !self.constrained {
                    continue;
                }
                let available = if symbol == ManaSymbol::Snow {
                    pools[class].snow_available(color)
                } else {
                    pools[class].available(color)
                };
                if available == 0 {
                    continue;
                }
                let mut next = pools.clone();
                let paid = if symbol == ManaSymbol::Snow {
                    next[class].spend_snow(color)
                } else {
                    next[class].spend(color, 1)
                };
                let mut next_snow = hard_snow;
                if symbol == ManaSymbol::Snow {
                    next_snow[class][color.index()] += 1;
                }
                if paid
                    && let Some(result) = self.assign(
                        next,
                        index + 1,
                        generic,
                        extra_quota,
                        if class == 0 {
                            spent(prefer, color)
                        } else {
                            prefer
                        },
                        next_snow,
                    )
                {
                    return Some(result);
                }
            }
        }
        if matches!(symbol, ManaSymbol::TwoOrColor(_)) {
            return self.assign(
                pools,
                index + 1,
                generic.checked_add(2)?,
                extra_quota.checked_add(if self.constrained { 2 } else { 0 })?,
                prefer,
                hard_snow,
            );
        }
        None
    }

    fn options(&self, symbol: ManaSymbol) -> SmallVec<[ManaColor; 6]> {
        let mut colors = match symbol {
            ManaSymbol::White => smallvec![ManaColor::White],
            ManaSymbol::Blue => smallvec![ManaColor::Blue],
            ManaSymbol::Black => smallvec![ManaColor::Black],
            ManaSymbol::Red => smallvec![ManaColor::Red],
            ManaSymbol::Green => smallvec![ManaColor::Green],
            ManaSymbol::Colorless => return smallvec![ManaColor::Colorless],
            ManaSymbol::Snow => return ManaColor::ALL.into_iter().collect(),
            ManaSymbol::Phyrexian(color) | ManaSymbol::TwoOrColor(color) => {
                smallvec![ManaColor::from_color(color)]
            }
            ManaSymbol::Hybrid(pair) | ManaSymbol::HybridPhyrexian(pair) => smallvec![
                ManaColor::from_color(pair.first()),
                ManaColor::from_color(pair.second())
            ],
            _ => unreachable!("only constrained symbols need color options"),
        };
        let literal = colors.clone();
        for actual in ManaColor::ALL {
            if !colors.contains(&actual)
                && literal
                    .iter()
                    .any(|&color| self.spending.permits(actual, color))
            {
                colors.push(actual);
            }
        }
        colors
    }

    fn generic(
        &self,
        mut pools: [ManaPool; 2],
        amount: u32,
        extra_quota: u32,
        prefer: [u32; 6],
        hard_snow: [[u32; 6]; 2],
    ) -> Option<SourcePayment> {
        for class in 0..2 {
            for color in ManaColor::ALL {
                let i = color.index();
                let snow = self.original_pools[class].snow_available(color);
                if self.receipts[class].snow[i] > snow
                    || self.receipts[class].plain[i] > self.original[class][i] - snow
                {
                    return None;
                }
            }
        }
        let resources = std::array::from_fn::<_, 2, _>(|class| {
            let available = amounts(&pools[class]);
            generic::Resources {
                available,
                required: std::array::from_fn(|color| {
                    let exact = hard_snow[class][color]
                        .max(self.receipts[class].snow[color])
                        .saturating_add(self.receipts[class].plain[color]);
                    self.required[class][color]
                        .max(exact)
                        .saturating_sub(self.original[class][color] - available[color])
                }),
            }
        });
        let allocation = generic::allocate_restricting_types(
            resources[0],
            resources[1],
            amount,
            self.quota.saturating_add(extra_quota),
            prefer,
            self.restriction.map(|r| (r.types, r.amount, true)),
        )?;
        let mut restricted_generic_paid = [0; 6];
        if let Some(restriction) = self.restriction {
            let mut remaining = restriction.amount;
            for (i, paid) in restricted_generic_paid.iter_mut().enumerate() {
                if restriction.types & (1 << i) != 0 {
                    let take = allocation.ordinary[i]
                        .saturating_add(allocation.restricted[i])
                        .min(remaining);
                    *paid = take;
                    remaining -= take;
                }
            }
            debug_assert_eq!(remaining, 0);
        }
        for (class, payment) in [allocation.ordinary, allocation.restricted]
            .into_iter()
            .enumerate()
        {
            for color in ManaColor::ALL {
                if !pools[class].spend(color, payment[color.index()]) {
                    return None;
                }
            }
        }
        // Colored/generic symbols may swap snow provenance within their actual
        // type. Literal snow pips cannot: keep their lower bound in the branch,
        // so an incompatible mandatory plain unit causes genuine backtracking.
        for class in 0..2 {
            let mut exact = self.original_pools[class].clone();
            for color in ManaColor::ALL {
                let i = color.index();
                let used = self.original[class][i] - pools[class].available(color);
                let snow_before = self.original_pools[class].snow_available(color);
                let minimum = hard_snow[class][i]
                    .max(self.receipts[class].snow[i])
                    .max(used.saturating_sub(self.original[class][i] - snow_before));
                let maximum = snow_before.min(used.checked_sub(self.receipts[class].plain[i])?);
                if minimum > maximum {
                    return None;
                }
                let used_snow =
                    (snow_before - pools[class].snow_available(color)).clamp(minimum, maximum);
                if !exact.spend(color, used - used_snow)
                    || !exact.spend_snow_units(color, used_snow)
                {
                    return None;
                }
            }
            pools[class] = exact;
        }
        let [remaining, restricted_remaining] = pools;
        Some(SourcePayment {
            remaining,
            restricted_remaining,
            restricted_generic_paid,
        })
    }
}
