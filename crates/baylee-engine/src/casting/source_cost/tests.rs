//! Child of `casting::source_cost`. Permission arithmetic, not price construction.
#[allow(clippy::wildcard_imports)]
use super::*;
fn generic_cost(amount: u32) -> ManaCost {
    ManaCost::from_symbol_generic(amount)
}
#[test]
fn allowance_matches_small_exact_price_component_reference() {
    for base in 0_u32..=7 {
        for eligible in 0_u32..=7 {
            for ordinary in 0_u32..=7 {
                for reduction in 0_u32..=24 {
                    for constrained in [false, true] {
                        let final_generic = (base + eligible + ordinary).saturating_sub(reduction);
                        let eligible_original = eligible + if constrained { base } else { 0 };
                        let allowance = SourceAllowance::base(constrained)
                            .adjusted(eligible, ordinary, reduction);
                        assert_eq!(allowance.constrained, constrained);
                        assert_eq!(
                            allowance.quota(&generic_cost(final_generic)),
                            eligible_original.min(final_generic),
                            "B={base} E={eligible} I={ordinary} R={reduction} constrained={constrained}"
                        );
                    }
                }
            }
        }
    }
}
#[test]
fn ordinary_selected_generic_payments_can_discharge_ordinary_floor_first() {
    for base in 0_u32..=5 {
        for eligible in 0_u32..=5 {
            for ordinary in 0_u32..=5 {
                for reduction in 0_u32..=16 {
                    for constrained in [false, true] {
                        let total = (base + eligible + ordinary).saturating_sub(reduction);
                        let allowance = SourceAllowance::base(constrained)
                            .adjusted(eligible, ordinary, reduction);
                        let original_quota = allowance.quota(&generic_cost(total));
                        let ordinary_floor = total - original_quota;
                        for selected in 0..=total {
                            let remaining = total - selected;
                            let residual_floor = ordinary_floor.saturating_sub(selected);
                            assert_eq!(
                                allowance.reduced(selected).quota(&generic_cost(remaining)),
                                remaining - residual_floor,
                                "selected ordinary generic units are attributed to ordinary demand first"
                            );
                        }
                    }
                }
            }
        }
    }
}
#[test]
fn selected_payment_reduction_is_associative_and_never_increases_absolute_quota() {
    for constrained in [false, true] {
        for ordinary in 0_u32..=5 {
            for eligible in 0_u32..=5 {
                for first in 0_u32..=8 {
                    for second in 0_u32..=8 {
                        let allowance =
                            SourceAllowance::base(constrained).adjusted(eligible, ordinary, 0);
                        assert_eq!(
                            allowance.reduced(first).reduced(second),
                            allowance.reduced(first + second)
                        );
                        let total = 5 + eligible + ordinary;
                        assert!(
                            allowance
                                .reduced(first)
                                .quota(&generic_cost(total.saturating_sub(first)))
                                <= allowance.quota(&generic_cost(total))
                        );
                    }
                }
            }
        }
    }
}
#[test]
fn max_allowance_preserves_existing_saturating_pipeline_and_ordinary_floor() {
    let values = [0, 1, u32::MAX - 1, u32::MAX];
    for base in values {
        for eligible in values {
            for ordinary in values {
                for reduction in values {
                    for constrained in [false, true] {
                        let final_generic = base
                            .saturating_add(eligible)
                            .saturating_add(ordinary)
                            .saturating_sub(reduction);
                        let allowance = SourceAllowance::base(constrained)
                            .adjusted(eligible, ordinary, reduction);
                        let expected = if constrained {
                            final_generic.saturating_sub(ordinary.saturating_sub(reduction))
                        } else {
                            eligible.min(final_generic)
                        };
                        assert_eq!(allowance.quota(&generic_cost(final_generic)), expected);
                        assert!(allowance.quota(&generic_cost(final_generic)) <= final_generic);
                        let paid = allowance.reduced(u32::MAX);
                        assert_eq!(paid.quota(&generic_cost(0)), 0);
                    }
                }
            }
        }
    }
    // Document overflow information loss in pricing; allowance cannot recover
    // dropped components and must not claim widened-u64 price semantics.
    let saturated = u32::MAX.saturating_add(1).saturating_sub(1);
    let widened = (u64::from(u32::MAX) + 1 - 1).min(u64::from(u32::MAX));
    assert_eq!(saturated, u32::MAX - 1);
    assert_ne!(u64::from(saturated), widened);
}
