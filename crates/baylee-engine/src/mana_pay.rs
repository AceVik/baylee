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

    // ---- Pinned: the two defects on #172. ----
    //
    // These assert what the code does *today*, which is wrong, so that the
    // commit repairing it has to come here and delete them. A limitation
    // recorded only in prose has no test that goes red when it stops being
    // true, and that is how it survives. **Breaking these is the success.**

    #[test]
    fn pinned_172_a_payable_pair_of_hybrids_is_refused() {
        // `{W/U}` is canonically first, wants white first, and takes the
        // only W; `{W/B}` then finds neither of its colours and the greedy
        // walk has no way back. U pays `{W/U}` and W pays `{W/B}`.
        let pool = pool_of(&[(ManaColor::White, 1), (ManaColor::Blue, 1)]);
        let cost = baylee_core::mana!("{W/U}{W/B}");
        assert!(
            !can_pay(&pool, &cost),
            "#172 is fixed: delete this pin and assert the payment instead"
        );
    }

    #[test]
    fn pinned_172_a_failed_payment_can_leave_the_pool_spent() {
        // `can_pay` reserves two generic for `{2/W}` and pays `{2/U}` with
        // the U; `pay` instead spends the U inside `{2/W}`'s fallback and
        // then cannot pay `{2/U}` at all. `pay_any` subtracts as it goes
        // and has no rollback, so the refusal costs the player the pool.
        let mut pool = pool_of(&[(ManaColor::Blue, 1), (ManaColor::Black, 2)]);
        let cost = baylee_core::mana!("{2/W}{2/U}");
        assert!(can_pay(&pool, &cost), "the probe says yes");
        assert!(!pay(&mut pool, &cost), "and the payment says no");
        assert_eq!(
            pool.total(),
            0,
            "#172 is fixed: the pool is intact (or the payment succeeded) \
             — delete this pin"
        );
    }
}
