//! Child of `mana_pay`: independent mixed-symbol source-class acceptance cases.
#[allow(clippy::wildcard_imports)]
use super::*;
fn units(entries: &[(ManaColor, u32, bool)]) -> ManaPool {
    let mut pool = ManaPool::new();
    for &(color, count, snow) in entries {
        if snow {
            pool.add_snow(color, count);
        } else {
            pool.add(color, count);
        }
    }
    pool
}
fn solve_source(
    normal: &ManaPool,
    restricted: &ManaPool,
    cost: &str,
    constrained: bool,
    quota: u32,
    spending: ManaSpending,
) -> Option<SourcePayment> {
    payment_with_source_credits(
        normal,
        &ManaCost::parse(cost),
        spending,
        [0; 6],
        None,
        [0; 6],
        SourceCredits {
            pool: restricted.clone(),
            required: [0; 6],
            generic_quota: quota,
            constrained,
        },
    )
}
#[test]
fn source_hybrid_must_choose_color_and_class_jointly() {
    let normal = units(&[(ManaColor::Blue, 1, false)]);
    let restricted = units(&[(ManaColor::Red, 1, false)]);
    for cost in ["{R/U}{U}", "{R/U/P}{U}"] {
        let paid = solve_source(&normal, &restricted, cost, true, 0, ManaSpending::EXACT).unwrap();
        assert!(paid.remaining.is_empty());
        assert!(paid.restricted_remaining.is_empty());
        assert!(
            solve_source(&normal, &restricted, cost, false, 1, ManaSpending::EXACT).is_none(),
            "generic quota cannot make restricted mana pay either constrained pip"
        );
    }
}
#[test]
fn eligible_two_or_color_fallback_adds_only_its_own_two_generic_units() {
    let normal = units(&[(ManaColor::Blue, 1, false)]);
    let restricted = units(&[(ManaColor::Red, 2, false)]);
    let paid = solve_source(
        &normal,
        &restricted,
        "{1}{2/W}",
        true,
        0,
        ManaSpending::EXACT,
    )
    .unwrap();
    assert!(paid.remaining.is_empty());
    assert!(paid.restricted_remaining.is_empty());
    assert!(
        solve_source(
            &ManaPool::new(),
            &units(&[(ManaColor::Red, 3, false)]),
            "{1}{2/W}",
            true,
            0,
            ManaSpending::EXACT
        )
        .is_none(),
        "fallback two belongs to eligible base; independent generic tax does not"
    );
    assert!(
        solve_source(
            &ManaPool::new(),
            &restricted,
            "{2/W}",
            true,
            0,
            ManaSpending::EXACT
        )
        .is_some()
    );
    assert!(
        solve_source(
            &ManaPool::new(),
            &restricted,
            "{2/W}",
            false,
            0,
            ManaSpending::EXACT
        )
        .is_none()
    );
}
#[test]
fn snow_assignment_backtracks_across_classes_without_spending_snow_on_the_wrong_pip() {
    let normal = units(&[(ManaColor::Red, 1, true)]);
    let restricted = units(&[(ManaColor::Red, 1, false)]);
    let paid = solve_source(&normal, &restricted, "{S}{R}", true, 0, ManaSpending::EXACT).unwrap();
    assert!(paid.remaining.is_empty());
    assert!(paid.restricted_remaining.is_empty());
    assert!(
        solve_source(
            &normal,
            &restricted,
            "{S}{R}",
            false,
            1,
            ManaSpending::ANY_COLOR
        )
        .is_none()
    );
    // Blue is visited before red by the symbol chooser: spending ordinary
    // snow blue on S first must fail, then retry restricted snow red for S.
    let backed = solve_source(
        &units(&[(ManaColor::Blue, 1, true)]),
        &units(&[(ManaColor::Red, 1, true)]),
        "{S}{U}",
        true,
        0,
        ManaSpending::EXACT,
    )
    .unwrap();
    assert!(backed.remaining.is_empty());
    assert!(backed.restricted_remaining.is_empty());
    let swapped =
        solve_source(&restricted, &normal, "{S}{R}", true, 0, ManaSpending::EXACT).unwrap();
    assert!(swapped.remaining.is_empty());
    assert!(swapped.restricted_remaining.is_empty());
}
#[test]
fn directed_spending_does_not_invert_permissions_or_widen_source_classes() {
    let mut spending = ManaSpending::EXACT;
    spending.allow(ManaColor::Red, ManaColor::Blue);
    let red = units(&[(ManaColor::Red, 1, false)]);
    let blue = units(&[(ManaColor::Blue, 1, false)]);
    assert!(solve_source(&red, &ManaPool::new(), "{U}", false, 0, spending).is_some());
    assert!(solve_source(&blue, &ManaPool::new(), "{R}", false, 0, spending).is_none());
    assert!(solve_source(&ManaPool::new(), &red, "{U}", true, 0, spending).is_some());
    assert!(solve_source(&ManaPool::new(), &red, "{U}", false, 1, spending).is_none());
    // Core allow() combines compatible permissions transitively; the source
    // solver must honor that supplied matrix, never infer reverse edges.
    spending.allow(ManaColor::Blue, ManaColor::Green);
    assert!(spending.permits(ManaColor::Red, ManaColor::Green));
    assert!(solve_source(&red, &ManaPool::new(), "{G}", false, 0, spending).is_some());
    assert!(
        solve_source(
            &units(&[(ManaColor::Green, 1, false)]),
            &ManaPool::new(),
            "{R}",
            false,
            0,
            spending
        )
        .is_none()
    );
}
#[test]
fn source_boundaries_sum_in_u64_and_failure_keeps_both_inputs_unchanged() {
    let normal = units(&[(ManaColor::Red, u32::MAX, false)]);
    let restricted = units(&[(ManaColor::Blue, u32::MAX, true)]);
    let before_normal = normal.clone();
    let before_restricted = restricted.clone();
    let cost = ManaCost::from_symbol_generic(u32::MAX);
    let paid = payment_with_source_credits(
        &normal,
        &cost,
        ManaSpending::EXACT,
        [0; 6],
        None,
        [0; 6],
        SourceCredits {
            pool: restricted.clone(),
            required: [0; 6],
            generic_quota: u32::MAX,
            constrained: false,
        },
    )
    .unwrap();
    assert_eq!(
        paid.remaining.total() + paid.restricted_remaining.total(),
        u64::from(u32::MAX)
    );
    assert!(
        solve_source(
            &normal,
            &restricted,
            "{C}",
            true,
            u32::MAX,
            ManaSpending::ANY_COLOR
        )
        .is_none()
    );
    assert_eq!(normal, before_normal);
    assert_eq!(restricted, before_restricted);
}
