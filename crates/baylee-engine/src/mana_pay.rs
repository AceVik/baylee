//! Mana payment: legality probe + auto-payment.
//!
//! Exact deterministic assignment reserves constrained symbols and backtracks
//! over hybrid, two-or-color and snow choices. Payment is transactional.
//! Phyrexian life is decided above this module and never here: an
//! activation settles each symbol first (`ManaCost::with_phyrexian_settled`,
//! asked by `Engine::start_activation`) and a spell's cast wizard does the
//! same at the head of its X stage (`Engine::ask_phyrexian`), so what
//! reaches this module is mana; an unsettled Phyrexian symbol is paid with
//! its colour.

use baylee_core::mana::{ManaColor, ManaCost, ManaPool, ManaSpending, ManaSymbol};
use smallvec::SmallVec;

mod generic;
mod source_credits;
pub(crate) use source_credits::{
    ReceiptRequirement, SourceCredits, SourcePayment, payment_with_source_credits,
    payment_with_source_receipts,
};
#[cfg(test)]
mod source_credit_extra;
#[cfg(test)]
mod source_credit_oracle;
#[cfg(test)]
mod source_receipt_oracle;

/// Actual mana colors permitted for a specified generic component.
/// Spending-as-though permissions never widen this set.
#[derive(Clone, Copy, Debug)]
pub struct GenericRestriction {
    /// Allowed actual colors.
    pub types: u8,
    /// Number of generic units subject to this restriction.
    pub amount: u32,
}

impl From<(ManaColor, u32)> for GenericRestriction {
    fn from((color, amount): (ManaColor, u32)) -> Self {
        Self {
            types: 1 << color.index(),
            amount,
        }
    }
}

/// Separate resources usable only for a bounded generic cost component.
/// They cannot satisfy colored, colorless, hybrid or snow symbols, even under
/// spending permissions. Required units are separate from ordinary obligations.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct GenericCredits {
    pub available: [u32; 6],
    pub required: [u32; 6],
    pub quota: u32,
}

impl GenericCredits {
    fn capacity(self) -> Option<u64> {
        let required: u64 = self.required.iter().map(|&n| u64::from(n)).sum();
        if required > u64::from(self.quota)
            || self
                .required
                .iter()
                .zip(self.available)
                .any(|(&need, have)| need > have)
        {
            return None;
        }
        Some(
            self.available
                .iter()
                .map(|&n| u64::from(n))
                .sum::<u64>()
                .min(u64::from(self.quota)),
        )
    }
}

/// Ordinary remainder and the actual types of generic-only resources consumed.
pub(crate) struct CreditedPayment {
    pub remaining: ManaPool,
    pub credits: [u32; 6],
}

/// Whether `pool` can pay `cost` at all (ignoring Phyrexian life).
#[must_use]
pub fn can_pay(pool: &ManaPool, cost: &ManaCost) -> bool {
    payment(pool, cost).is_some()
}

/// Mycosynth Lattice: mana may be spent as though it were any color.
#[must_use]
pub fn can_pay_wild(pool: &ManaPool, cost: &ManaCost) -> bool {
    can_pay_with(pool, cost, ManaSpending::ANY_COLOR)
}

/// Pays a cost in wild mode; a failed payment leaves the pool unchanged.
pub fn pay_wild(pool: &mut ManaPool, cost: &ManaCost) -> bool {
    pay_with(pool, cost, ManaSpending::ANY_COLOR)
}

/// Pays `cost` exactly; leaves the pool untouched on failure (#172).
pub fn pay(pool: &mut ManaPool, cost: &ManaCost) -> bool {
    let Some(paid) = payment(pool, cost) else {
        return false;
    };
    *pool = paid;
    true
}

fn payment(pool: &ManaPool, cost: &ManaCost) -> Option<ManaPool> {
    payment_preferring(pool, cost, false, [0; 6])
}

/// Whether `pool` pays `cost` under these spending permissions.
#[must_use]
pub fn can_pay_with(pool: &ManaPool, cost: &ManaCost, spending: ManaSpending) -> bool {
    payment_with(pool, cost, spending, [0; 6]).is_some()
}

/// Pays using the actual mana types allowed by `spending`. Failure is atomic.
pub fn pay_with(pool: &mut ManaPool, cost: &ManaCost, spending: ManaSpending) -> bool {
    let Some(paid) = payment_with(pool, cost, spending, [0; 6]) else {
        return false;
    };
    *pool = paid;
    true
}

/// The pool after paying `cost`, spending the `prefer`red units first
/// wherever the payment has a choice, or `None` when it cannot be paid.
///
/// `prefer[c]` counts units of colour `c` already in `pool` that the caller
/// would rather see spent. That is the restricted mana a spell or an
/// ability may use: Cavern of Souls' mana beside a land's, so that the
/// Cavern's unit pays and its rider lands. `wild` is Mycosynth Lattice,
/// applied exactly as [`pay_wild`] applies it.
///
/// The preference only reorders. At every choice the same alternatives are
/// tried, a preferred one first: which half of a hybrid, which colour pays a
/// snow pip, which colours the generic remainder takes. So this succeeds on
/// exactly the pools [`can_pay`] and [`can_pay_wild`] accept, and a payer
/// built on it never refuses what a probe built on those offered. With
/// `prefer == [0; 6]` it is [`payment`], unit for unit.
#[must_use]
pub fn payment_preferring(
    pool: &ManaPool,
    cost: &ManaCost,
    wild: bool,
    prefer: [u32; 6],
) -> Option<ManaPool> {
    payment_with(
        pool,
        cost,
        if wild {
            ManaSpending::ANY_COLOR
        } else {
            ManaSpending::EXACT
        },
        prefer,
    )
}

/// Solves payment with directed spending permissions, preferring the
/// specified actual units while preserving their colors and snow provenance.
#[must_use]
pub fn payment_with(
    pool: &ManaPool,
    cost: &ManaCost,
    spending: ManaSpending,
    prefer: [u32; 6],
) -> Option<ManaPool> {
    payment_restricting_generic(pool, cost, spending, prefer, None)
}

/// Solves a payment with a restriction on the actual type of some generic
/// mana. Spending permissions cannot change which actual units satisfy it.
#[must_use]
pub fn payment_restricting_generic(
    pool: &ManaPool,
    cost: &ManaCost,
    spending: ManaSpending,
    prefer: [u32; 6],
    restriction: Option<GenericRestriction>,
) -> Option<ManaPool> {
    payment_consuming(pool, cost, spending, prefer, restriction, [0; 6])
}

/// Solve while consuming at least `required` actual units of each mana type.
/// Those units may pay any part of the cost that permits their actual type.
/// A failed assignment backtracks across flexible symbols; generic amounts
/// are charged in six bounded steps, regardless of their magnitude.
#[must_use]
pub fn payment_consuming(
    pool: &ManaPool,
    cost: &ManaCost,
    spending: ManaSpending,
    prefer: [u32; 6],
    restriction: Option<GenericRestriction>,
    required: [u32; 6],
) -> Option<ManaPool> {
    payment_with_generic_credits(
        pool,
        cost,
        spending,
        prefer,
        restriction,
        required,
        GenericCredits::default(),
    )
    .map(|payment| {
        debug_assert_eq!(payment.credits, [0; 6]);
        payment.remaining
    })
}

/// Joint assignment of ordinary symbols and a bounded generic-only component.
/// Actual-type generic reservations are ordinary obligations; they are kept
/// disjoint from credits for an independently imposed generic component.
#[must_use]
pub(crate) fn payment_with_generic_credits(
    pool: &ManaPool,
    cost: &ManaCost,
    spending: ManaSpending,
    prefer: [u32; 6],
    restriction: Option<GenericRestriction>,
    required: [u32; 6],
    credits: GenericCredits,
) -> Option<CreditedPayment> {
    let consumption = Consumption {
        original: ManaColor::ALL.map(|color| pool.available(color)),
        required,
        credits,
        credit_capacity: credits.capacity()?,
        restriction,
    };
    if required
        .iter()
        .zip(consumption.original)
        .any(|(&need, available)| need > available)
    {
        return None;
    }
    let remaining = *cost;
    let mut symbols: SmallVec<[ManaSymbol; 16]> = remaining.symbols().collect();
    symbols.sort_by_key(|s| match s {
        ManaSymbol::Hybrid(_) | ManaSymbol::HybridPhyrexian(_) | ManaSymbol::TwoOrColor(_) => {
            (1, 0)
        }
        ManaSymbol::Generic(_) => (2, 0),
        // A literal colorless requirement remains constrained under Lattice.
        // Reserve it before colored pips can use all the colorless mana.
        ManaSymbol::Colorless | ManaSymbol::Snow => (0, 0),
        _ => (0, 1),
    });
    assign(pool.clone(), &symbols, 0, prefer, spending, &consumption)
}

struct Consumption {
    original: [u32; 6],
    required: [u32; 6],
    credits: GenericCredits,
    credit_capacity: u64,
    restriction: Option<GenericRestriction>,
}

impl Consumption {
    fn pay_generic(
        &self,
        mut pool: ManaPool,
        amount: u32,
        prefer: [u32; 6],
    ) -> Option<CreditedPayment> {
        let available = ManaColor::ALL.map(|color| pool.available(color));
        let required = std::array::from_fn(|i| {
            self.required[i].saturating_sub(self.original[i] - available[i])
        });
        let allocation = generic::allocate_restricting_types(
            generic::Resources {
                available,
                required,
            },
            generic::Resources {
                available: self.credits.available,
                required: self.credits.required,
            },
            amount,
            self.credits.quota,
            prefer,
            self.restriction.map(|r| (r.types, r.amount, false)),
        )?;
        for color in ManaColor::ALL {
            if !pool.spend(color, allocation.ordinary[color.index()]) {
                return None;
            }
        }
        Some(CreditedPayment {
            remaining: pool,
            credits: allocation.restricted,
        })
    }
}

/// `prefer` with one unit of `color` spent.
fn spent(mut prefer: [u32; 6], color: ManaColor) -> [u32; 6] {
    prefer[color.index()] = prefer[color.index()].saturating_sub(1);
    prefer
}

/// `options` with the colours that still have preferred units first, each
/// half in its given order.
fn preferred_first(options: &[ManaColor], prefer: [u32; 6]) -> SmallVec<[ManaColor; 6]> {
    let wanted = |c: &&ManaColor| prefer[c.index()] > 0;
    options
        .iter()
        .filter(wanted)
        .chain(options.iter().filter(|c| !wanted(c)))
        .copied()
        .collect()
}

/// Reserve constrained symbols first, backtracking on flexible choices.
/// Generic fallback is deferred so it cannot consume a later symbol's color.
fn assign(
    pool: ManaPool,
    symbols: &[ManaSymbol],
    generic: u32,
    prefer: [u32; 6],
    spending: ManaSpending,
    consumption: &Consumption,
) -> Option<CreditedPayment> {
    // Reject an undersized pool before exploring equivalent color choices.
    // A two-or-color pip needs at least one mana; deferred generic fallback
    // is counted separately. Restricted entries are not plain spendable units.
    let minimum = symbols.iter().fold(generic, |sum, symbol| {
        sum.saturating_add(match symbol {
            ManaSymbol::Generic(n) => *n,
            ManaSymbol::Variable(_) | ManaSymbol::HalfGeneric | ManaSymbol::Infinite => 0,
            _ => 1,
        })
    });
    let available: u64 = ManaColor::ALL
        .into_iter()
        .map(|c| u64::from(pool.available(c)))
        .sum();
    if u64::from(minimum) > available + consumption.credit_capacity {
        return None;
    }
    let Some((symbol, rest)) = symbols.split_first() else {
        return consumption.pay_generic(pool, generic, prefer);
    };
    let colors: &[ManaColor] = match *symbol {
        ManaSymbol::White => &[ManaColor::White],
        ManaSymbol::Blue => &[ManaColor::Blue],
        ManaSymbol::Black => &[ManaColor::Black],
        ManaSymbol::Red => &[ManaColor::Red],
        ManaSymbol::Green => &[ManaColor::Green],
        ManaSymbol::Colorless => &[ManaColor::Colorless],
        ManaSymbol::Snow => {
            for color in preferred_first(&ManaColor::ALL, prefer) {
                let mut trial = pool.clone();
                if trial.spend_snow(color)
                    && let Some(paid) = assign(
                        trial,
                        rest,
                        generic,
                        spent(prefer, color),
                        spending,
                        consumption,
                    )
                {
                    return Some(paid);
                }
            }
            return None;
        }
        ManaSymbol::Phyrexian(c) | ManaSymbol::TwoOrColor(c) => &[ManaColor::from_color(c)],
        ManaSymbol::Hybrid(p) | ManaSymbol::HybridPhyrexian(p) => &[
            ManaColor::from_color(p.first()),
            ManaColor::from_color(p.second()),
        ],
        ManaSymbol::Generic(n) => {
            return assign(
                pool,
                rest,
                generic.saturating_add(n),
                prefer,
                spending,
                consumption,
            );
        }
        ManaSymbol::Variable(_) | ManaSymbol::HalfGeneric | ManaSymbol::Infinite => {
            return assign(pool, rest, generic, prefer, spending, consumption);
        }
    };
    // Literal colors first preserves the ordinary payer's choices. Added
    // options name actual mana, so accounting never turns white into red.
    let mut options: SmallVec<[ManaColor; 6]> = colors.iter().copied().collect();
    for actual in ManaColor::ALL {
        if !options.contains(&actual)
            && colors
                .iter()
                .any(|&required| spending.permits(actual, required))
        {
            options.push(actual);
        }
    }
    for color in preferred_first(&options, prefer) {
        let mut trial = pool.clone();
        if trial.spend(color, 1)
            && let Some(paid) = assign(
                trial,
                rest,
                generic,
                spent(prefer, color),
                spending,
                consumption,
            )
        {
            return Some(paid);
        }
    }
    if matches!(symbol, ManaSymbol::TwoOrColor(_)) {
        return assign(
            pool,
            rest,
            generic.saturating_add(2),
            prefer,
            spending,
            consumption,
        );
    }
    None
}

/// Whether one subset of an actual payment can pay `subset` while the
/// remaining units can pay the fixed component. The subset is read as its
/// actual mana types; spending permissions of the original payment apply
/// only to the fixed component. No choice loops over a generic amount.
#[must_use]
pub(crate) fn paid_subset_can_pay(
    paid: &ManaPool,
    subset: &ManaCost,
    limit: u32,
    fixed: &ManaCost,
    fixed_spending: ManaSpending,
) -> bool {
    let mut symbols: SmallVec<[(ManaSymbol, bool); 16]> = subset
        .symbols()
        .map(|symbol| (symbol, true))
        .chain(fixed.symbols().map(|symbol| (symbol, false)))
        .collect();
    symbols.sort_by_key(|(symbol, _)| matches!(symbol, ManaSymbol::Generic(_)));
    split_assign(paid.clone(), &symbols, [0; 2], limit, fixed_spending)
}

fn split_assign(
    pool: ManaPool,
    symbols: &[(ManaSymbol, bool)],
    generic: [u32; 2],
    limit: u32,
    fixed_spending: ManaSpending,
) -> bool {
    if generic[0] > limit {
        return false;
    }
    let Some((&(symbol, subset), rest)) = symbols.split_first() else {
        let available: u64 = ManaColor::ALL
            .into_iter()
            .map(|color| u64::from(pool.available(color)))
            .sum();
        return available >= u64::from(generic[0]) + u64::from(generic[1]);
    };
    let index = usize::from(!subset);
    let mut generic = generic;
    match symbol {
        ManaSymbol::Generic(n) => {
            generic[index] = generic[index].saturating_add(n);
            return split_assign(pool, rest, generic, limit, fixed_spending);
        }
        ManaSymbol::Variable(_) => return split_assign(pool, rest, generic, limit, fixed_spending),
        ManaSymbol::HalfGeneric | ManaSymbol::Infinite => return false,
        _ => {}
    }
    let next_limit = if subset {
        let Some(n) = limit.checked_sub(1) else {
            return false;
        };
        n
    } else {
        limit
    };
    let accepts = |actual: ManaColor| {
        let spending = if subset {
            ManaSpending::EXACT
        } else {
            fixed_spending
        };
        match symbol {
            ManaSymbol::Snow => true,
            ManaSymbol::Colorless => actual == ManaColor::Colorless,
            ManaSymbol::White => spending.permits(actual, ManaColor::White),
            ManaSymbol::Blue => spending.permits(actual, ManaColor::Blue),
            ManaSymbol::Black => spending.permits(actual, ManaColor::Black),
            ManaSymbol::Red => spending.permits(actual, ManaColor::Red),
            ManaSymbol::Green => spending.permits(actual, ManaColor::Green),
            ManaSymbol::Phyrexian(color) | ManaSymbol::TwoOrColor(color) => {
                spending.permits(actual, ManaColor::from_color(color))
            }
            ManaSymbol::Hybrid(pair) | ManaSymbol::HybridPhyrexian(pair) => {
                spending.permits(actual, ManaColor::from_color(pair.first()))
                    || spending.permits(actual, ManaColor::from_color(pair.second()))
            }
            _ => false,
        }
    };
    for color in ManaColor::ALL {
        if !accepts(color) {
            continue;
        }
        let mut next = pool.clone();
        let spent = if symbol == ManaSymbol::Snow {
            next.spend_snow(color)
        } else {
            next.spend(color, 1)
        };
        if spent && split_assign(next, rest, generic, next_limit, fixed_spending) {
            return true;
        }
    }
    if matches!(symbol, ManaSymbol::TwoOrColor(_)) {
        generic[index] = generic[index].saturating_add(2);
        return split_assign(pool, rest, generic, limit, fixed_spending);
    }
    false
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod credit_tests;
