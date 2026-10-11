//! Spending permissions scoped to one component of a total mana payment.
use super::{ManaCost, ManaSymbol, Slot, slot};

impl ManaCost {
    /// The specified component still payable with mana after the total cost's
    /// Phyrexian choices. Identical additional Phyrexian symbols take life first.
    /// This reserves the most flexible valid component for any-type spending.
    #[must_use]
    pub fn component_after_phyrexian(&self, component: &Self, life: u32) -> Self {
        let mut result = Self::ZERO;
        let mut offset = 0_u32;
        for (symbol, count) in self.runs() {
            let Slot::Kind(index) = slot(symbol) else {
                if let Some(amount) = component.generic {
                    result.add(ManaSymbol::Generic(amount.min(self.generic_total())), 1);
                }
                continue;
            };
            let component_count = component.counts[index].min(count);
            let settled = match symbol {
                ManaSymbol::Phyrexian(color) => Some(ManaSymbol::of_color(color)),
                ManaSymbol::HybridPhyrexian(pair) => Some(ManaSymbol::Hybrid(pair)),
                _ => None,
            };
            if let Some(settled) = settled {
                // Only the first 32 Phyrexian symbols have a life-choice bit.
                // Count a whole run at once, including runs extending past it.
                let bits = u32::from(count).min(32_u32.saturating_sub(offset));
                let mask = if bits == 0 {
                    0
                } else {
                    u32::MAX >> (32 - bits)
                };
                let by_life = (life.checked_shr(offset).unwrap_or(0) & mask).count_ones();
                offset = offset.saturating_add(u32::from(count));
                let component_life = by_life.saturating_sub(u32::from(count - component_count));
                result.add(
                    settled,
                    u32::from(component_count).saturating_sub(component_life),
                );
            } else {
                result.add(symbol, u32::from(component_count));
            }
        }
        result
    }

    /// Convert only eligible colored, hybrid and colorless symbols to generic
    /// payment obligations. Call after cost determination and reductions: this
    /// changes how mana is spent, never the spell's actual cost or mana value.
    /// Snow still requires mana produced by a snow source.
    #[must_use]
    pub fn with_any_type_component(&self, component: &Self) -> Self {
        let mut result = Self::ZERO;
        for (symbol, count) in self.runs() {
            let eligible = match (symbol, slot(symbol)) {
                (
                    ManaSymbol::White
                    | ManaSymbol::Blue
                    | ManaSymbol::Black
                    | ManaSymbol::Red
                    | ManaSymbol::Green
                    | ManaSymbol::Colorless
                    | ManaSymbol::Hybrid(_)
                    | ManaSymbol::TwoOrColor(_),
                    Slot::Kind(index),
                ) => component.counts[index].min(count),
                _ => 0,
            };
            result.add(symbol, u32::from(count - eligible));
            result.add(
                ManaSymbol::Generic(u32::from(eligible)),
                u32::from(eligible != 0),
            );
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::{Color, ColorPair};

    fn expanded_conversion(total: &ManaCost, component: &ManaCost) -> ManaCost {
        let mut available: Vec<_> = component.symbols().collect();
        let mut result = ManaCost::ZERO;
        for symbol in total.symbols() {
            let convertible = matches!(
                symbol,
                ManaSymbol::White
                    | ManaSymbol::Blue
                    | ManaSymbol::Black
                    | ManaSymbol::Red
                    | ManaSymbol::Green
                    | ManaSymbol::Colorless
                    | ManaSymbol::Hybrid(_)
                    | ManaSymbol::TwoOrColor(_)
            );
            if convertible && let Some(index) = available.iter().position(|other| *other == symbol)
            {
                available.remove(index);
                result.add(ManaSymbol::Generic(1), 1);
            } else {
                result.add(symbol, 1);
            }
        }
        result
    }

    #[test]
    fn north_star_leaves_colored_additional_costs_and_snow_intact() {
        let base = ManaCost::parse("{2}{G}{C}{W/U}{2/B}{S}");
        let extra = ManaCost::parse("{B}{G}{C}{W/U}{2/B}{S}");
        let total = base.combine(&extra);
        assert_eq!(
            total.with_any_type_component(&base),
            ManaCost::parse("{6}{B}{G}{C}{W/U}{2/B}{S}{S}")
        );
        assert_eq!(total.with_any_type_component(&ManaCost::ZERO), total);
        // A converted {2/B} requires one arbitrary mana (its colored option),
        // but converting payment never changes the original two mana value.
        assert_eq!(
            ManaCost::parse("{2/B}").with_any_type_component(&ManaCost::parse("{2/B}")),
            ManaCost::parse("{1}")
        );
        assert_eq!(ManaCost::parse("{2/B}").cmc(), 2);
    }

    #[test]
    fn north_star_conversion_matches_expanded_symbols_for_every_kind_pair() {
        // All 42 symbol kinds, including those that must not be converted;
        // unequal multiplicities also test clamping to the actual total.
        for first in super::super::KIND_SYMBOLS {
            for second in super::super::KIND_SYMBOLS {
                for count in 0..=3 {
                    let mut total = ManaCost::parse("{3}");
                    total.add(first, 2);
                    total.add(second, 1);
                    let mut component = ManaCost::parse("{9}");
                    component.add(first, count);
                    component.add(second, 1);
                    assert_eq!(
                        total.with_any_type_component(&component),
                        expanded_conversion(&total, &component),
                        "total={total:?}, component={component:?}"
                    );
                }
            }
        }
    }

    fn maximum_component_paid(total: u32, component: u32, life: u32) -> u32 {
        (0_u32..(1 << total))
            .filter(|allocation| allocation.count_ones() == component.min(total))
            .map(|allocation| (allocation & !life).count_ones())
            .max()
            .unwrap_or(0)
    }

    #[test]
    fn north_star_phyrexian_component_matches_all_allocations_of_identical_pips() {
        for symbol in [
            ManaSymbol::Phyrexian(Color::White),
            ManaSymbol::HybridPhyrexian(ColorPair::new(Color::Blue, Color::Red)),
        ] {
            let settled = match symbol {
                ManaSymbol::Phyrexian(color) => ManaSymbol::of_color(color),
                ManaSymbol::HybridPhyrexian(pair) => ManaSymbol::Hybrid(pair),
                _ => unreachable!(),
            };
            for total in 0..=6 {
                for component in 0..=7 {
                    let mut cost = ManaCost::ZERO;
                    cost.add(symbol, total);
                    let mut base = ManaCost::ZERO;
                    base.add(symbol, component);
                    for life in 0..(1 << total) {
                        let mut expected = ManaCost::ZERO;
                        expected.add(settled, maximum_component_paid(total, component, life));
                        assert_eq!(
                            cost.component_after_phyrexian(&base, life),
                            expected,
                            "cost={cost:?}, component={base:?}, life={life:#b}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn north_star_phyrexian_offsets_preserve_additional_colored_requirements() {
        let total = ManaCost::parse("{5}{B}{W/P}{W/P}{W/P}{G/P}{G/P}{U/R/P}{U/R/P}{U/R/P}");
        let base = ManaCost::parse("{2}{W/P}{W/P}{G/P}{U/R/P}{U/R/P}");
        for life in 0..256 {
            let mut expected = ManaCost::parse("{2}");
            expected.add(ManaSymbol::White, maximum_component_paid(3, 2, life & 7));
            expected.add(
                ManaSymbol::Green,
                maximum_component_paid(2, 1, (life >> 3) & 3),
            );
            expected.add(
                ManaSymbol::Hybrid(ColorPair::new(Color::Blue, Color::Red)),
                maximum_component_paid(3, 2, (life >> 5) & 7),
            );
            let actual = total.component_after_phyrexian(&base, life);
            assert_eq!(actual, expected, "life={life:#b}");
            let settled = total.with_phyrexian_settled(life);
            assert_eq!(
                settled.with_any_type_component(&actual),
                expanded_conversion(&settled, &expected)
            );
        }
    }

    #[test]
    fn north_star_phyrexian_masks_stop_at_32_without_losing_later_symbols() {
        let mut total = ManaCost::ZERO;
        total.add(ManaSymbol::Phyrexian(Color::White), 40);
        total.add(ManaSymbol::Phyrexian(Color::Green), 3);
        let mut base = ManaCost::ZERO;
        base.add(ManaSymbol::Phyrexian(Color::White), 20);
        base.add(ManaSymbol::Phyrexian(Color::Green), 2);
        let mut expected = ManaCost::parse("{G}{G}");
        expected.add(ManaSymbol::White, 8);
        assert_eq!(total.component_after_phyrexian(&base, u32::MAX), expected);
        // The mask also ends exactly at a run boundary.
        total = ManaCost::ZERO;
        total.add(ManaSymbol::Phyrexian(Color::White), 32);
        total.add(ManaSymbol::Phyrexian(Color::Green), 3);
        assert_eq!(
            total.component_after_phyrexian(&base, u32::MAX),
            ManaCost::parse("{G}{G}")
        );
    }

    #[test]
    fn north_star_component_generic_is_clamped_independently_of_other_symbols() {
        assert_eq!(
            ManaCost::parse("{3}{G}").component_after_phyrexian(&ManaCost::parse("{7}{W}"), 0),
            ManaCost::parse("{3}")
        );
        assert_eq!(
            ManaCost::parse("{0}{G}").component_after_phyrexian(&ManaCost::parse("{2}"), 0),
            ManaCost::parse("{0}")
        );
        assert_eq!(
            ManaCost::parse("{G}").component_after_phyrexian(&ManaCost::parse("{2}"), 0),
            ManaCost::ZERO
        );
    }
}
