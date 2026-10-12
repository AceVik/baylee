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
    restriction: Option<(ManaColor, u32)>,
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
    restriction: Option<(ManaColor, u32)>,
    required: [u32; 6],
) -> Option<ManaPool> {
    let consumption = Consumption {
        original: ManaColor::ALL.map(|color| pool.available(color)),
        required,
    };
    if required
        .iter()
        .zip(consumption.original)
        .any(|(&need, available)| need > available)
    {
        return None;
    }
    let mut reserved = pool.clone();
    let mut remaining = *cost;
    if let Some((color, amount)) = restriction {
        if amount > cost.generic_total() || !reserved.spend(color, amount) {
            return None;
        }
        remaining = cost.with_less_generic(amount);
    }
    let mut symbols: Vec<_> = remaining.symbols().collect();
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
    assign(reserved, &symbols, 0, prefer, spending, &consumption)
}

struct Consumption {
    original: [u32; 6],
    required: [u32; 6],
}

impl Consumption {
    fn pay_generic(&self, pool: &mut ManaPool, mut amount: u32, mut prefer: [u32; 6]) -> bool {
        for color in ManaColor::ALL {
            let i = color.index();
            let already = self.original[i] - pool.available(color);
            let remaining = self.required[i].saturating_sub(already);
            let Some(left) = amount.checked_sub(remaining) else {
                return false;
            };
            if !pool.spend(color, remaining) {
                return false;
            }
            amount = left;
            prefer[i] = prefer[i].saturating_sub(remaining);
        }
        pay_any(pool, amount, prefer)
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
    mut pool: ManaPool,
    symbols: &[ManaSymbol],
    generic: u32,
    prefer: [u32; 6],
    spending: ManaSpending,
    consumption: &Consumption,
) -> Option<ManaPool> {
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
    if u64::from(minimum) > available {
        return None;
    }
    let Some((symbol, rest)) = symbols.split_first() else {
        return consumption
            .pay_generic(&mut pool, generic, prefer)
            .then_some(pool);
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

/// Pays `n` generic out of whatever is left, the preferred units first.
///
/// It succeeds exactly when the pool holds `n` units in all, whatever the
/// preference: the first pass only decides which units the second pass
/// does not have to find.
fn pay_any(pool: &mut ManaPool, n: u32, prefer: [u32; 6]) -> bool {
    let mut remaining = n;
    for color in ManaColor::ALL {
        let take = pool
            .available(color)
            .min(prefer[color.index()])
            .min(remaining);
        pool.spend(color, take);
        remaining -= take;
    }
    for color in ManaColor::ALL {
        let take = pool.available(color).min(remaining);
        pool.spend(color, take);
        remaining -= take;
    }
    remaining == 0
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
    let mut symbols: Vec<_> = subset
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
