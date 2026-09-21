//! Mana payment: legality probe + auto-payment.
//!
//! Exact deterministic assignment reserves constrained symbols and backtracks
//! over hybrid, two-or-color and snow choices. Payment is transactional;
//! Phyrexian life is handled by the casting wizard rather than this module.

use baylee_core::mana::{ManaColor, ManaCost, ManaPool, ManaSymbol};

/// Whether `pool` can pay `cost` at all (ignoring Phyrexian life).
#[must_use]
pub fn can_pay(pool: &ManaPool, cost: &ManaCost) -> bool {
    payment(pool, cost).is_some()
}

/// Mycosynth Lattice: mana may be spent as though it were any color.
#[must_use]
pub fn can_pay_wild(pool: &ManaPool, cost: &ManaCost) -> bool {
    payment(pool, &wild_cost(cost)).is_some()
}

/// Pays a cost in wild mode; a failed payment leaves the pool unchanged.
pub fn pay_wild(pool: &mut ManaPool, cost: &ManaCost) -> bool {
    pay(pool, &wild_cost(cost))
}

fn wild_cost(cost: &ManaCost) -> ManaCost {
    let mut result = ManaCost::ZERO;
    for symbol in cost.symbols() {
        result = result.combine(&ManaCost::from_symbol(if symbol == ManaSymbol::Snow {
            symbol
        } else {
            ManaSymbol::Generic(symbol.cmc_contribution())
        }));
    }
    result
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
    let mut symbols: Vec<_> = cost.symbols().collect();
    symbols.sort_by_key(|s| match s {
        ManaSymbol::Hybrid(_) | ManaSymbol::HybridPhyrexian(_) | ManaSymbol::TwoOrColor(_) => 1,
        ManaSymbol::Generic(_) => 2,
        _ => 0,
    });
    assign(pool.clone(), &symbols, 0)
}

/// Reserve constrained symbols first, backtracking on flexible choices.
/// Generic fallback is deferred so it cannot consume a later symbol's color.
fn assign(mut pool: ManaPool, symbols: &[ManaSymbol], generic: u32) -> Option<ManaPool> {
    let Some((symbol, rest)) = symbols.split_first() else {
        return pay_any(&mut pool, generic).then_some(pool);
    };
    let colors: &[ManaColor] = match *symbol {
        ManaSymbol::White => &[ManaColor::White],
        ManaSymbol::Blue => &[ManaColor::Blue],
        ManaSymbol::Black => &[ManaColor::Black],
        ManaSymbol::Red => &[ManaColor::Red],
        ManaSymbol::Green => &[ManaColor::Green],
        ManaSymbol::Colorless => &[ManaColor::Colorless],
        ManaSymbol::Snow => {
            for color in ManaColor::ALL {
                let mut trial = pool.clone();
                if trial.spend_snow(color)
                    && let Some(paid) = assign(trial, rest, generic)
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
        ManaSymbol::Generic(n) => return assign(pool, rest, generic.saturating_add(n)),
        ManaSymbol::Variable(_) | ManaSymbol::HalfGeneric | ManaSymbol::Infinite => {
            return assign(pool, rest, generic);
        }
    };
    for &color in colors {
        let mut trial = pool.clone();
        if trial.spend(color, 1)
            && let Some(paid) = assign(trial, rest, generic)
        {
            return Some(paid);
        }
    }
    if matches!(symbol, ManaSymbol::TwoOrColor(_)) {
        return assign(pool, rest, generic.saturating_add(2));
    }
    None
}

fn pay_any(pool: &mut ManaPool, n: u32) -> bool {
    let mut remaining = n;
    for color in ManaColor::ALL {
        let take = pool
            .available(color)
            .min(u16::try_from(remaining).unwrap_or(u16::MAX));
        pool.spend(color, take);
        remaining -= u32::from(take);
    }
    remaining == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_158_snow_is_provenance_not_color() {
        for color in ManaColor::ALL {
            let mut ordinary = pool_of(&[(color, 1)]);
            let before = ordinary.clone();
            assert!(!pay(&mut ordinary, &baylee_core::mana!("{S}")));
            assert!(!pay_wild(&mut ordinary, &baylee_core::mana!("{S}")));
            assert_eq!(ordinary, before);
            let mut snow = ManaPool::new();
            snow.add_snow(color, 1);
            assert!(pay(&mut snow, &baylee_core::mana!("{S}")));
            assert!(snow.is_empty());
        }
        let mut mixed = pool_of(&[(ManaColor::Blue, 1)]);
        mixed.add_snow(ManaColor::Blue, 1);
        assert!(pay(&mut mixed, &baylee_core::mana!("{U}{S}")));
        assert!(mixed.is_empty());
    }

    #[test]
    fn issue_172_hybrid_assignment_backtracks() {
        let mut pool = pool_of(&[(ManaColor::White, 1), (ManaColor::Blue, 1)]);
        let cost = baylee_core::mana!("{W/U}{W/B}");
        assert!(can_pay(&pool, &cost));
        assert!(pay(&mut pool, &cost));
        assert!(pool.is_empty());
    }

    #[test]
    fn issue_172_two_or_reserves_later_colors() {
        let mut pool = pool_of(&[(ManaColor::Blue, 1), (ManaColor::Black, 2)]);
        let cost = baylee_core::mana!("{2/W}{2/U}");
        assert!(pay(&mut pool, &cost));
        assert!(pool.is_empty());
    }

    #[test]
    fn issue_172_probe_and_payment_agree_and_failure_is_atomic() {
        let costs = [
            baylee_core::mana!("{W/U}{W/B}"),
            baylee_core::mana!("{2/W}{2/U}"),
            baylee_core::mana!("{1}{W/U}{B}"),
            baylee_core::mana!("{W/U/P}{W/B}"),
        ];
        for white in 0..=3 {
            for blue in 0..=3 {
                for black in 0..=3 {
                    let before = pool_of(&[
                        (ManaColor::White, white),
                        (ManaColor::Blue, blue),
                        (ManaColor::Black, black),
                    ]);
                    for cost in &costs {
                        let mut pool = before.clone();
                        let possible = can_pay(&pool, cost);
                        assert_eq!(pay(&mut pool, cost), possible);
                        if !possible {
                            assert_eq!(pool, before);
                        }
                    }
                }
            }
        }
    }

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

    /// A pool with the named colours, written the way a test reads.
    fn pool_of(counts: &[(ManaColor, u16)]) -> ManaPool {
        let mut pool = ManaPool::new();
        for &(color, n) in counts {
            pool.add(color, n);
        }
        pool
    }

    /// The colours left in a pool, so an assertion can name what survived
    /// rather than only how much did — two pools of one mana are not the
    /// same pool, and a payment that spends the wrong colour passes every
    /// `total()` check.
    fn remaining(pool: &ManaPool) -> Vec<(ManaColor, u16)> {
        ManaColor::ALL
            .iter()
            .filter_map(|&c| match pool.available(c) {
                0 => None,
                n => Some((c, n)),
            })
            .collect()
    }

    #[test]
    fn a_coloured_symbol_is_paid_from_its_own_colour() {
        let mut pool = pool_of(&[(ManaColor::Green, 1), (ManaColor::Red, 1)]);
        assert!(pay(&mut pool, &baylee_core::mana!("{G}")));
        assert_eq!(remaining(&pool), vec![(ManaColor::Red, 1)]);
    }

    #[test]
    fn the_coloured_symbols_are_paid_before_the_generic_ones() {
        // The discriminator: `ManaColor::ALL` starts at white, so a generic
        // pip paid first would take the W this cost needs and leave the G,
        // and the payment would fail with the pool able to cover it.
        let mut pool = pool_of(&[(ManaColor::White, 1), (ManaColor::Green, 1)]);
        assert!(can_pay(&pool, &baylee_core::mana!("{1}{W}")));
        assert!(pay(&mut pool, &baylee_core::mana!("{1}{W}")));
        assert_eq!(pool.total(), 0);
    }

    #[test]
    fn a_generic_pip_takes_any_colour_that_is_left() {
        let mut pool = pool_of(&[(ManaColor::Black, 3)]);
        assert!(pay(&mut pool, &baylee_core::mana!("{2}")));
        assert_eq!(remaining(&pool), vec![(ManaColor::Black, 1)]);
    }

    #[test]
    fn a_colourless_symbol_is_not_a_generic_one() {
        // CR 107.4c: `{C}` is a colourless *requirement*, and coloured mana
        // cannot pay it. The reverse is not true, which the next test says.
        let coloured = pool_of(&[(ManaColor::White, 2)]);
        assert!(!can_pay(&coloured, &baylee_core::mana!("{C}")));
        let mut colourless = pool_of(&[(ManaColor::Colorless, 1)]);
        assert!(pay(&mut colourless, &baylee_core::mana!("{C}")));
        assert_eq!(colourless.total(), 0);
    }

    #[test]
    fn colourless_mana_pays_a_generic_pip() {
        let mut pool = pool_of(&[(ManaColor::Colorless, 2)]);
        assert!(pay(&mut pool, &baylee_core::mana!("{2}")));
        assert_eq!(pool.total(), 0);
    }

    #[test]
    fn a_hybrid_takes_its_second_colour_when_the_first_is_absent() {
        let mut pool = pool_of(&[(ManaColor::Blue, 1)]);
        assert!(can_pay(&pool, &baylee_core::mana!("{W/U}")));
        assert!(pay(&mut pool, &baylee_core::mana!("{W/U}")));
        assert_eq!(pool.total(), 0);
    }

    #[test]
    fn one_hybrid_pair_printed_twice_uses_both_of_its_halves() {
        // The shape every hybrid card in this pool actually prints —
        // `{3}{W/B}{W/B}`, `{2}{G/W}{G/W}{G/W}` — as opposed to two
        // *different* pairs, which is the case `can_pay` gets wrong.
        let mut pool = pool_of(&[(ManaColor::White, 1), (ManaColor::Black, 1)]);
        assert!(can_pay(&pool, &baylee_core::mana!("{W/B}{W/B}")));
        assert!(pay(&mut pool, &baylee_core::mana!("{W/B}{W/B}")));
        assert_eq!(pool.total(), 0);
    }

    #[test]
    fn a_hybrid_beside_a_plain_symbol_pays_the_plain_one_first() {
        // Thopter Foundry's `{W/B}{U}`: the plain `{U}` sorts ahead, so the
        // hybrid is left choosing from what the plain symbol did not want.
        let mut pool = pool_of(&[(ManaColor::White, 1), (ManaColor::Blue, 1)]);
        assert!(pay(&mut pool, &baylee_core::mana!("{W/B}{U}")));
        assert_eq!(pool.total(), 0);
    }

    #[test]
    fn a_two_or_colour_symbol_is_paid_by_its_colour_for_one() {
        let mut pool = pool_of(&[(ManaColor::White, 1), (ManaColor::Red, 1)]);
        assert!(pay(&mut pool, &baylee_core::mana!("{2/W}")));
        assert_eq!(remaining(&pool), vec![(ManaColor::Red, 1)]);
    }

    #[test]
    fn a_two_or_colour_symbol_falls_back_to_two_of_anything() {
        let mut pool = pool_of(&[(ManaColor::Blue, 2)]);
        assert!(can_pay(&pool, &baylee_core::mana!("{2/W}")));
        assert!(pay(&mut pool, &baylee_core::mana!("{2/W}")));
        assert_eq!(pool.total(), 0);

        // One is not two, and there is no half-price.
        let short = pool_of(&[(ManaColor::Blue, 1)]);
        assert!(!can_pay(&short, &baylee_core::mana!("{2/W}")));
    }

    #[test]
    fn phyrexian_is_refused_here_when_its_colour_is_absent() {
        // The module pays Phyrexian *with mana* and says so; the life
        // half is a decision made above this layer, so a pool that cannot
        // produce the colour is a refusal rather than a two-life payment.
        let empty = ManaPool::new();
        assert!(!can_pay(&empty, &baylee_core::mana!("{W/P}")));
        let mut coloured = pool_of(&[(ManaColor::White, 1)]);
        assert!(pay(&mut coloured, &baylee_core::mana!("{W/P}")));
        assert_eq!(coloured.total(), 0);
    }

    #[test]
    fn an_announced_x_costs_nothing_at_this_layer() {
        // `{X}` is skipped by both halves: the number the player announced
        // reaches the pool as generic pips from the caller, so a cost still
        // carrying its `{X}` must not be charged for it twice.
        let mut pool = pool_of(&[(ManaColor::Red, 1)]);
        assert!(can_pay(&pool, &baylee_core::mana!("{X}{R}")));
        assert!(pay(&mut pool, &baylee_core::mana!("{X}{R}")));
        assert_eq!(pool.total(), 0);
    }

    #[test]
    fn an_empty_cost_is_free_and_an_empty_pool_can_pay_it() {
        let mut pool = ManaPool::new();
        assert!(can_pay(&pool, &baylee_core::mana!("{0}")));
        assert!(pay(&mut pool, &baylee_core::mana!("{0}")));
    }

    #[test]
    fn a_cost_the_pool_cannot_cover_leaves_it_untouched() {
        // `pay` asks `can_pay` first, so a refusal it can see coming costs
        // the player nothing. The pool is compared colour by colour: a
        // check on the total alone would pass a payment that spent the
        // wrong mana and put the same amount back.
        let before = pool_of(&[(ManaColor::Green, 1), (ManaColor::White, 1)]);
        let mut pool = pool_of(&[(ManaColor::Green, 1), (ManaColor::White, 1)]);
        assert!(!pay(&mut pool, &baylee_core::mana!("{G}{G}")));
        assert_eq!(remaining(&pool), remaining(&before));
    }

    #[test]
    fn asking_whether_a_cost_is_payable_spends_nothing() {
        let pool = pool_of(&[(ManaColor::Green, 2)]);
        assert!(can_pay(&pool, &baylee_core::mana!("{1}{G}")));
        assert!(can_pay(&pool, &baylee_core::mana!("{1}{G}")));
        assert_eq!(remaining(&pool), vec![(ManaColor::Green, 2)]);
    }

    #[test]
    fn two_payments_out_of_one_pool_drain_it_exactly() {
        let mut pool = pool_of(&[(ManaColor::Red, 2), (ManaColor::Green, 1)]);
        assert!(pay(&mut pool, &baylee_core::mana!("{R}")));
        assert_eq!(pool.total(), 2);
        assert!(pay(&mut pool, &baylee_core::mana!("{1}{G}")));
        assert_eq!(pool.total(), 0);
        assert!(!can_pay(&pool, &baylee_core::mana!("{1}")));
    }

    #[test]
    fn wild_mode_spends_any_colour_for_any_symbol() {
        // Mycosynth Lattice: the whole cost reduces to its mana value.
        let mut pool = pool_of(&[(ManaColor::Blue, 3)]);
        let cost = baylee_core::mana!("{W}{W}{W}");
        assert!(can_pay_wild(&pool, &cost));
        assert!(!can_pay(&pool, &cost), "only wild mode may do this");
        assert!(pay_wild(&mut pool, &cost));
        assert_eq!(pool.total(), 0);
    }

    #[test]
    fn wild_mode_still_counts() {
        let mut pool = pool_of(&[(ManaColor::Blue, 2)]);
        let cost = baylee_core::mana!("{2}{W}");
        assert!(!can_pay_wild(&pool, &cost));
        assert!(!pay_wild(&mut pool, &cost));
        assert_eq!(pool.total(), 2, "a refused wild payment spends nothing");
    }
}
