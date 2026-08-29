//! Mana payment: legality probe + auto-payment.
//!
//! S2 implements exact auto-payment (colored symbols first, then hybrid,
//! then generic) and treats Phyrexian as mana-paid. The full payment-plan
//! solver (meaningfully distinct plans → `ChoiceRequest::PayMana`) is M2.

use baylee_core::mana::{ManaColor, ManaCost, ManaPool, ManaSymbol};

/// Whether `pool` can pay `cost` at all (ignoring Phyrexian life).
#[must_use]
pub fn can_pay(pool: &ManaPool, cost: &ManaCost) -> bool {
    let mut need: Vec<ManaSymbol> = cost.symbols().collect();
    // Pay exact colors first, then hybrid/two-or, then generic.
    need.sort_by_key(|s| match s {
        ManaSymbol::White
        | ManaSymbol::Blue
        | ManaSymbol::Black
        | ManaSymbol::Red
        | ManaSymbol::Green
        | ManaSymbol::Colorless
        | ManaSymbol::Snow => 0,
        ManaSymbol::Hybrid(_) | ManaSymbol::TwoOrColor(_) | ManaSymbol::Phyrexian(_) => 1,
        _ => 2,
    });
    let mut available: Vec<ManaColor> = Vec::new();
    for color in ManaColor::ALL {
        available.extend(std::iter::repeat_n(color, pool.available(color) as usize));
    }
    let mut used = vec![false; available.len()];
    let mut generic_needed = 0u32;

    'symbols: for symbol in need {
        let wanted: &[ManaColor] = match symbol {
            ManaSymbol::White => &[ManaColor::White],
            ManaSymbol::Blue => &[ManaColor::Blue],
            ManaSymbol::Black => &[ManaColor::Black],
            ManaSymbol::Red => &[ManaColor::Red],
            ManaSymbol::Green => &[ManaColor::Green],
            ManaSymbol::Colorless | ManaSymbol::Snow => &[ManaColor::Colorless],
            ManaSymbol::Phyrexian(c) | ManaSymbol::TwoOrColor(c) => &[ManaColor::from_color(c)],
            ManaSymbol::Hybrid(p) | ManaSymbol::HybridPhyrexian(p) => &[
                ManaColor::from_color(p.first()),
                ManaColor::from_color(p.second()),
            ],
            ManaSymbol::Generic(n) => {
                generic_needed += n;
                continue;
            }
            ManaSymbol::Variable(_) | ManaSymbol::HalfGeneric | ManaSymbol::Infinite => {
                continue;
            }
        };
        for &color in wanted {
            if let Some(i) = available
                .iter()
                .enumerate()
                .position(|(j, c)| !used[j] && *c == color)
            {
                used[i] = true;
                continue 'symbols;
            }
        }
        // Phyrexian/two-or-color/hybrid can fall back to generic amounts.
        match symbol {
            ManaSymbol::TwoOrColor(_) => generic_needed += 2,
            _ => return false,
        }
    }
    let remaining = used.iter().filter(|u| !**u).count() as u32;
    remaining >= generic_needed
}

/// Pays `cost` from `pool` if possible (auto-payment).
///
/// Returns `true` and mutates the pool on success; leaves the pool
/// untouched and returns `false` on failure.
#[must_use]
/// Mycosynth Lattice: every mana spends as any color — the whole cost
/// reduces to its cmc against the pool total.
pub fn can_pay_wild(pool: &ManaPool, cost: &ManaCost) -> bool {
    pool.total() >= cost.cmc()
}

/// Pays a cost in wild mode (any mana for any symbol).
pub fn pay_wild(pool: &mut ManaPool, cost: &ManaCost) -> bool {
    if !can_pay_wild(pool, cost) {
        return false;
    }
    let mut remaining = cost.cmc();
    for color in baylee_core::mana::ManaColor::ALL {
        if remaining == 0 {
            break;
        }
        let have = pool.available(color);
        let take = have.min(remaining as u16);
        if take > 0 {
            pool.spend(color, take);
            remaining -= u32::from(take);
        }
    }
    remaining == 0
}

/// Pays a cost from the pool (exact colors first, flexible last).
pub fn pay(pool: &mut ManaPool, cost: &ManaCost) -> bool {
    if !can_pay(pool, cost) {
        return false;
    }
    // Colored symbols first, then hybrid, generic last — the flexible mana
    // is spent where it is actually needed.
    let mut symbols: Vec<ManaSymbol> = cost.symbols().collect();
    symbols.sort_by_key(|s| match s {
        ManaSymbol::White
        | ManaSymbol::Blue
        | ManaSymbol::Black
        | ManaSymbol::Red
        | ManaSymbol::Green
        | ManaSymbol::Colorless
        | ManaSymbol::Snow => 0,
        ManaSymbol::Hybrid(_) | ManaSymbol::TwoOrColor(_) | ManaSymbol::Phyrexian(_) => 1,
        _ => 2,
    });
    for symbol in symbols {
        match symbol {
            ManaSymbol::White
            | ManaSymbol::Blue
            | ManaSymbol::Black
            | ManaSymbol::Red
            | ManaSymbol::Green => {
                let color = match symbol {
                    ManaSymbol::White => ManaColor::White,
                    ManaSymbol::Blue => ManaColor::Blue,
                    ManaSymbol::Black => ManaColor::Black,
                    ManaSymbol::Red => ManaColor::Red,
                    ManaSymbol::Green => ManaColor::Green,
                    _ => unreachable!(),
                };
                if !pool.spend(color, 1) {
                    return false;
                }
            }
            ManaSymbol::Colorless | ManaSymbol::Snow => {
                if !pool.spend(ManaColor::Colorless, 1) {
                    return false;
                }
            }
            ManaSymbol::Phyrexian(c) | ManaSymbol::TwoOrColor(c) => {
                let color = ManaColor::from_color(c);
                if !pool.spend(color, 1) {
                    let fallback: &[ManaColor] = match symbol {
                        ManaSymbol::TwoOrColor(_) => &ManaColor::ALL,
                        _ => &[],
                    };
                    if fallback.is_empty() || !pay_any(pool, fallback, 2) {
                        return false;
                    }
                }
            }
            ManaSymbol::Hybrid(p) | ManaSymbol::HybridPhyrexian(p) => {
                let first = ManaColor::from_color(p.first());
                let second = ManaColor::from_color(p.second());
                if !pool.spend(first, 1) && !pool.spend(second, 1) {
                    return false;
                }
            }
            ManaSymbol::Generic(n) => {
                if !pay_any(pool, &ManaColor::ALL, n) {
                    return false;
                }
            }
            ManaSymbol::Variable(_) | ManaSymbol::HalfGeneric | ManaSymbol::Infinite => {}
        }
    }
    true
}

fn pay_any(pool: &mut ManaPool, colors: &[ManaColor], n: u32) -> bool {
    let mut remaining = n;
    for &color in colors {
        while remaining > 0 && pool.spend(color, 1) {
            remaining -= 1;
        }
    }
    remaining == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pays_simple_costs() {
        let mut pool = ManaPool::new();
        pool.add(ManaColor::Blue, 2);
        pool.add(ManaColor::Red, 1);
        assert!(pay(&mut pool, &baylee_core::mana!("{1}{U}")));
        assert_eq!(pool.total(), 1); // 3 in pool − 2 paid
        assert!(!can_pay(&pool, &baylee_core::mana!("{U}{U}")));
    }

    #[test]
    fn pays_hybrid_and_generic() {
        let mut pool = ManaPool::new();
        pool.add(ManaColor::White, 1);
        pool.add(ManaColor::Green, 1);
        assert!(can_pay(&pool, &baylee_core::mana!("{W/U}")));
        assert!(pay(&mut pool, &baylee_core::mana!("{W/U}")));
        assert_eq!(pool.total(), 1);
        assert!(pay(&mut pool, &baylee_core::mana!("{1}")));
    }
}
