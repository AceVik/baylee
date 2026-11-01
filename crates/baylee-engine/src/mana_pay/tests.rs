use super::*;

#[test]
fn mandatory_mana_consumption_backtracks_and_precedes_optional_preferences() {
    let mut pool = ManaPool::new();
    pool.add(ManaColor::White, 1);
    pool.add(ManaColor::Blue, 100_000);
    let mut required = [0; 6];
    required[ManaColor::Blue.index()] = 1;
    let mut prefer = [0; 6];
    prefer[ManaColor::White.index()] = 1;
    let paid = payment_consuming(
        &pool,
        &ManaCost::parse("{W/U}"),
        ManaSpending::EXACT,
        prefer,
        None,
        required,
    )
    .unwrap();
    assert_eq!(
        paid.available(ManaColor::White),
        1,
        "failed white branch backtracks to blue"
    );
    assert_eq!(paid.available(ManaColor::Blue), 99_999);
    required[ManaColor::Blue.index()] = 90_000;
    let paid = payment_consuming(
        &pool,
        &ManaCost::parse("{90000}"),
        ManaSpending::EXACT,
        prefer,
        None,
        required,
    )
    .unwrap();
    assert_eq!(
        paid.available(ManaColor::White),
        1,
        "required blue precedes preferred white"
    );
    assert_eq!(paid.available(ManaColor::Blue), 10_000);
    assert!(
        payment_consuming(
            &pool,
            &ManaCost::parse("{89999}"),
            ManaSpending::EXACT,
            prefer,
            None,
            required
        )
        .is_none()
    );
    assert_eq!(
        pool.available(ManaColor::Blue),
        100_000,
        "failed searches are transactional"
    );
}

#[test]
fn a_large_generic_payment_and_its_restricted_part_use_full_width() {
    let mut pool = ManaPool::new();
    pool.add(ManaColor::Blue, 160_000);
    pool.add(ManaColor::Red, 2);
    let mut preferred = [0; 6];
    preferred[ManaColor::Blue.index()] = 100_000;
    let paid = payment_restricting_generic(
        &pool,
        &ManaCost::parse("{140000}{R}"),
        ManaSpending::EXACT,
        preferred,
        Some((ManaColor::Blue, 90_000).into()),
    )
    .expect("large type-constrained generic payment fits");
    assert_eq!(paid.available(ManaColor::Blue), 20_000);
    assert_eq!(paid.available(ManaColor::Red), 1);
    let receipt = pool.payment_receipt(&paid).unwrap();
    assert_eq!(receipt.total(), 140_001);
    assert!(paid_subset_can_pay(
        &receipt,
        &ManaCost::parse("{140000}"),
        140_000,
        &ManaCost::parse("{R}"),
        ManaSpending::EXACT,
    ));
}

#[test]
fn full_width_pool_totals_do_not_overflow_solver_bounds() {
    let mut pool = ManaPool::new();
    pool.add(ManaColor::Blue, u32::MAX);
    pool.add(ManaColor::Green, u32::MAX);
    let paid = payment(&pool, &ManaCost::from_symbol_generic(u32::MAX)).unwrap();
    assert_eq!(paid.total(), u64::from(u32::MAX));
    assert!(paid_subset_can_pay(
        &pool,
        &ManaCost::from_symbol_generic(u32::MAX),
        u32::MAX,
        &ManaCost::from_symbol_generic(u32::MAX),
        ManaSpending::EXACT,
    ));
}

#[test]
fn paid_subset_reserves_fixed_colored_costs_and_actual_types() {
    let paid = pool_of(&[(ManaColor::Green, 1), (ManaColor::Colorless, 1)]);
    assert!(!paid_subset_can_pay(
        &paid,
        &ManaCost::parse("{G}"),
        1,
        &ManaCost::parse("{G}"),
        ManaSpending::EXACT
    ));
    assert!(paid_subset_can_pay(
        &paid,
        &ManaCost::parse("{1}"),
        1,
        &ManaCost::parse("{G}"),
        ManaSpending::EXACT
    ));
    assert!(!paid_subset_can_pay(
        &paid,
        &ManaCost::parse("{2}"),
        1,
        &ManaCost::ZERO,
        ManaSpending::EXACT
    ));
    assert!(!paid_subset_can_pay(
        &paid,
        &ManaCost::parse("{U}"),
        1,
        &ManaCost::ZERO,
        ManaSpending::ANY_COLOR
    ));
    let paid = pool_of(&[(ManaColor::Green, 1), (ManaColor::Blue, 1)]);
    assert!(paid_subset_can_pay(
        &paid,
        &ManaCost::parse("{G/U}"),
        1,
        &ManaCost::parse("{G}"),
        ManaSpending::EXACT
    ));
    let mut snow = ManaPool::new();
    snow.add_snow(ManaColor::Green, 1);
    snow.add(ManaColor::Colorless, 1);
    assert!(paid_subset_can_pay(
        &snow,
        &ManaCost::parse("{S}"),
        1,
        &ManaCost::parse("{1}"),
        ManaSpending::EXACT
    ));
    assert!(!paid_subset_can_pay(
        &snow,
        &ManaCost::parse("{S}"),
        1,
        &ManaCost::parse("{G}"),
        ManaSpending::EXACT
    ));
}

fn white_as_red() -> ManaSpending {
    let mut spending = ManaSpending::EXACT;
    spending.allow(ManaColor::White, ManaColor::Red);
    spending
}

#[test]
fn directed_spending_preserves_actual_colors_and_other_requirements() {
    let white = pool_of(&[(ManaColor::White, 1)]);
    for cost in ["{R}", "{R/G}", "{2/R}", "{R/P}", "{R/G/P}"] {
        let mut pool = white.clone();
        assert!(
            pay_with(&mut pool, &ManaCost::parse(cost), white_as_red()),
            "{cost}"
        );
        assert!(pool.is_empty());
    }
    for cost in ["{U}", "{B}", "{G}", "{C}", "{S}", "{W}{R}"] {
        let mut pool = white.clone();
        assert!(
            !pay_with(&mut pool, &ManaCost::parse(cost), white_as_red()),
            "{cost}"
        );
        assert_eq!(pool, white, "a failed payment is atomic");
    }
    let mut red = pool_of(&[(ManaColor::Red, 1)]);
    assert!(!pay_with(&mut red, &ManaCost::parse("{W}"), white_as_red()));

    let pool = pool_of(&[(ManaColor::White, 2), (ManaColor::Red, 1)]);
    let mut prefer = [0; 6];
    prefer[ManaColor::White.index()] = 1;
    let paid = payment_with(&pool, &ManaCost::parse("{R}"), white_as_red(), prefer).unwrap();
    assert_eq!(
        remaining(&paid),
        [(ManaColor::White, 1), (ManaColor::Red, 1)]
    );
}

#[test]
fn directed_spending_reserves_exact_colors_and_snow() {
    let mut pool = pool_of(&[(ManaColor::White, 1), (ManaColor::Red, 1)]);
    assert!(pay_with(
        &mut pool,
        &ManaCost::parse("{W}{R}"),
        white_as_red()
    ));
    assert!(pool.is_empty());

    let mut pool = pool_of(&[(ManaColor::White, 1)]);
    pool.add_snow(ManaColor::White, 1);
    assert!(pay_with(
        &mut pool,
        &ManaCost::parse("{R}{S}"),
        white_as_red()
    ));
    assert!(pool.is_empty());
    let mut pool = pool_of(&[(ManaColor::White, 2)]);
    assert!(!pay_with(
        &mut pool,
        &ManaCost::parse("{R}{S}"),
        white_as_red()
    ));
    assert_eq!(pool.total(), 2);
}

#[test]
fn directed_spending_probe_payment_and_preferences_agree() {
    for white in 0..=2 {
        for red in 0..=2 {
            for green in 0..=2 {
                let before = pool_of(&[
                    (ManaColor::White, white),
                    (ManaColor::Red, red),
                    (ManaColor::Green, green),
                ]);
                for text in [
                    "{W}{R}",
                    "{X}{R}",
                    "{1}{R/G}{W}",
                    "{2/R}{2/G}",
                    "{R/P}{W/R/P}",
                    "{S}{R}",
                    "{C}{R}",
                ] {
                    let cost = ManaCost::parse(text).with_x(2);
                    let possible = can_pay_with(&before, &cost, white_as_red());
                    for color in ManaColor::ALL {
                        let mut prefer = [0; 6];
                        prefer[color.index()] = 2;
                        assert_eq!(
                            payment_with(&before, &cost, white_as_red(), prefer).is_some(),
                            possible,
                            "{text}"
                        );
                    }
                    let mut pool = before.clone();
                    assert_eq!(pay_with(&mut pool, &cost, white_as_red()), possible);
                    if !possible {
                        assert_eq!(pool, before);
                    }
                }
            }
        }
    }
}

#[test]
fn any_color_spending_preserves_colorless_and_pays_twobrid_with_one_unit() {
    let pool = pool_of(&[(ManaColor::White, 1)]);
    assert!(!can_pay_wild(&pool, &ManaCost::parse("{C}")));
    let mut pool = pool_of(&[(ManaColor::Colorless, 1)]);
    assert!(pay_wild(&mut pool, &ManaCost::parse("{2/R}")));
    assert!(pool.is_empty());
    let mut pool = ManaPool::new();
    for color in ManaColor::ALL {
        pool.add(color, 2);
    }
    let before = pool.clone();
    let too_many = ManaCost::parse("{R}").combine_n(&ManaCost::parse("{R}"), 12);
    assert!(!pay_wild(&mut pool, &too_many));
    assert_eq!(
        pool, before,
        "an undersized rainbow pool is refused atomically"
    );
}

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
fn pool_of(counts: &[(ManaColor, u32)]) -> ManaPool {
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
fn remaining(pool: &ManaPool) -> Vec<(ManaColor, u32)> {
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

/// **A preference reorders and never decides.** Restricted mana is paid
/// by preferring its units, and a preference that could turn a payable
/// cost into a refused one would refuse a spell `LegalActions` offered.
/// So over a grid of pools, costs and preferences, the preferring payer
/// answers exactly what `can_pay` answers, and spends exactly the cost's
/// mana value.
#[test]
fn a_preference_never_changes_whether_a_cost_is_paid() {
    let costs = [
        baylee_core::mana!("{1}{U}"),
        baylee_core::mana!("{2}{G}"),
        baylee_core::mana!("{W/U}{W/B}"),
        baylee_core::mana!("{G/U}{1}"),
        baylee_core::mana!("{C}{1}"),
        baylee_core::mana!("{3}"),
        baylee_core::mana!("{U}{U}{G}"),
    ];
    let colors = [ManaColor::Blue, ManaColor::Green, ManaColor::Colorless];
    let mut checked = 0;
    for blue in 0..=2 {
        for green in 0..=2 {
            for colorless in 0..=2 {
                let pool = pool_of(&[
                    (ManaColor::Blue, blue),
                    (ManaColor::Green, green),
                    (ManaColor::Colorless, colorless),
                ]);
                for cost in &costs {
                    let possible = can_pay(&pool, cost);
                    for wanted in colors {
                        for n in 0..=pool.available(wanted) {
                            let mut prefer = [0_u32; 6];
                            prefer[wanted.index()] = n;
                            let paid = payment_preferring(&pool, cost, false, prefer);
                            assert_eq!(paid.is_some(), possible, "{cost} from {pool:?}");
                            if let Some(paid) = paid {
                                assert_eq!(
                                    pool.total() - paid.total(),
                                    u64::from(cost.cmc()),
                                    "{cost} from {pool:?} preferring {prefer:?}",
                                );
                            }
                            checked += 1;
                        }
                    }
                }
            }
        }
    }
    assert!(checked > 500, "the grid is the evidence: {checked}");
}

/// With nothing preferred it is the plain payment, unit for unit.
#[test]
fn with_nothing_preferred_it_is_the_plain_payment() {
    let pool = pool_of(&[
        (ManaColor::White, 1),
        (ManaColor::Blue, 2),
        (ManaColor::Green, 1),
    ]);
    for cost in [
        baylee_core::mana!("{1}{U}"),
        baylee_core::mana!("{W/U}{2}"),
        baylee_core::mana!("{G}{1}"),
    ] {
        assert_eq!(
            payment_preferring(&pool, &cost, false, [0; 6]),
            payment(&pool, &cost)
        );
    }
}

/// The generic part takes the preferred colour first, which is what puts
/// a Cavern's unit into a spell's `{1}` rather than a Forest's.
#[test]
fn the_generic_part_takes_the_preferred_colour_first() {
    let pool = pool_of(&[(ManaColor::Green, 1), (ManaColor::Blue, 1)]);
    let mut prefer = [0; 6];
    prefer[ManaColor::Green.index()] = 1;
    let paid = payment_preferring(&pool, &baylee_core::mana!("{1}"), false, prefer).unwrap();
    assert_eq!(remaining(&paid), vec![(ManaColor::Blue, 1)]);
    prefer = [0; 6];
    prefer[ManaColor::Blue.index()] = 1;
    let paid = payment_preferring(&pool, &baylee_core::mana!("{1}"), false, prefer).unwrap();
    assert_eq!(remaining(&paid), vec![(ManaColor::Green, 1)]);
}

/// A hybrid symbol pays with its preferred half where it has the choice.
#[test]
fn a_hybrid_pays_with_its_preferred_half() {
    let pool = pool_of(&[(ManaColor::White, 1), (ManaColor::Blue, 1)]);
    let mut prefer = [0; 6];
    prefer[ManaColor::Blue.index()] = 1;
    let paid = payment_preferring(&pool, &baylee_core::mana!("{W/U}"), false, prefer).unwrap();
    assert_eq!(remaining(&paid), vec![(ManaColor::White, 1)]);
}
#[test]
fn generic_actual_color_restriction_survives_spending_permissions() {
    let cost = baylee_core::mana!("{2}{B}");
    let red = pool_of(&[(ManaColor::Red, 3)]);
    assert!(payment_with(&red, &cost, ManaSpending::ANY_COLOR, [0; 6]).is_some());
    assert!(
        payment_restricting_generic(
            &red,
            &cost,
            ManaSpending::ANY_COLOR,
            [0; 6],
            Some((ManaColor::Black, 2).into()),
        )
        .is_none()
    );
    let mixed = pool_of(&[(ManaColor::Black, 2), (ManaColor::Red, 1)]);
    let paid = payment_restricting_generic(
        &mixed,
        &cost,
        ManaSpending::ANY_COLOR,
        [0; 6],
        Some((ManaColor::Black, 2).into()),
    )
    .unwrap();
    assert_eq!(paid.total(), 0);
    assert_eq!(mixed.total(), 3, "probing cannot spend the original pool");
    assert!(
        payment_restricting_generic(
            &mixed,
            &cost,
            ManaSpending::EXACT,
            [0; 6],
            Some((ManaColor::Black, 2).into()),
        )
        .is_none(),
        "the printed black pip needs a third black unit without permission"
    );
}
