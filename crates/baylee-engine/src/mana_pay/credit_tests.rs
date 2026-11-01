use super::*;

fn pool(entries: &[(ManaColor, u32, bool)]) -> ManaPool {
    let mut pool = ManaPool::new();
    for &(color, amount, snow) in entries {
        if snow {
            pool.add_snow(color, amount);
        } else {
            pool.add(color, amount);
        }
    }
    pool
}

fn credit(color: ManaColor, amount: u32, quota: u32) -> GenericCredits {
    let mut available = [0; 6];
    available[color.index()] = amount;
    GenericCredits {
        available,
        quota,
        ..GenericCredits::default()
    }
}

fn solve(pool: &ManaPool, cost: &str, credits: GenericCredits) -> Option<CreditedPayment> {
    payment_with_generic_credits(
        pool,
        &ManaCost::parse(cost),
        ManaSpending::EXACT,
        [0; 6],
        None,
        [0; 6],
        credits,
    )
}

#[test]
fn generic_credits_preserve_constrained_symbols_and_failed_searches_are_atomic() {
    use ManaColor::{Blue, Colorless, Red};
    let ordinary = pool(&[(Blue, 1, false)]);
    let before = ordinary.clone();
    let credits = credit(Red, 1, 1);
    let paid = solve(&ordinary, "{1}{U}", credits).unwrap();
    assert!(paid.remaining.is_empty());
    assert_eq!(paid.credits[Red.index()], 1);
    for cost in ["{1}{R}", "{1}{C}", "{1}{S}", "{1}{R/G}"] {
        assert!(solve(&ordinary, cost, credits).is_none(), "{cost}");
    }
    assert_eq!(ordinary, before);
    assert!(solve(&pool(&[(Colorless, 1, false)]), "{1}{C}", credits).is_some());
    assert!(solve(&pool(&[(Blue, 1, true)]), "{1}{S}", credits).is_some());
    assert!(solve(&ordinary, "{2/W}", credit(Red, 1, 0)).is_none());
    assert!(solve(&pool(&[(Blue, 2, false)]), "{1}{2/W}", credits).is_some());
}

#[test]
fn generic_credits_backtrack_to_satisfy_separate_mandatory_ordinary_units() {
    use ManaColor::{Blue, Red, White};
    let ordinary = pool(&[(White, 1, false), (Blue, 1, false)]);
    let mut required = [0; 6];
    required[Blue.index()] = 1;
    let mut credits = credit(Red, 1, 1);
    credits.required[Red.index()] = 1;
    let paid = payment_with_generic_credits(
        &ordinary,
        &ManaCost::parse("{1}{W/U}"),
        ManaSpending::EXACT,
        [0; 6],
        None,
        required,
        credits,
    )
    .unwrap();
    assert_eq!(paid.remaining.available(White), 1);
    assert_eq!(paid.remaining.available(Blue), 0);
    assert_eq!(paid.credits[Red.index()], 1);
    credits.quota = 0;
    assert!(solve(&ordinary, "{2}", credits).is_none());
}

#[test]
fn spending_permissions_do_not_turn_generic_credits_into_colored_mana() {
    use ManaColor::{Blue, Red};
    for spending in [ManaSpending::ANY_COLOR, ManaSpending::ANY_TYPE] {
        assert!(
            payment_with_generic_credits(
                &pool(&[(Blue, 1, false)]),
                &ManaCost::parse("{R}{R}"),
                spending,
                [0; 6],
                None,
                [0; 6],
                credit(Red, 1, 2),
            )
            .is_none()
        );
    }
}

#[test]
fn actual_type_reservations_remain_disjoint_from_generic_only_credits() {
    use ManaColor::{Blue, Red};
    // A red cost credit is not actual red mana for X, even if blue may
    // pay the fixed red pip through an independent spending permission.
    for spending in [
        ManaSpending::EXACT,
        ManaSpending::ANY_COLOR,
        ManaSpending::ANY_TYPE,
    ] {
        let ordinary = if spending == ManaSpending::EXACT {
            Red
        } else {
            Blue
        };
        assert!(
            payment_with_generic_credits(
                &pool(&[(ordinary, 1, false)]),
                &ManaCost::parse("{1}{R}"),
                spending,
                [0; 6],
                Some((Red, 1).into()),
                [0; 6],
                credit(Red, 1, 1),
            )
            .is_none()
        );
    }
    let credits = credit(Blue, 1, 1);
    assert!(
        payment_with_generic_credits(
            &pool(&[(Red, 1, false)]),
            &ManaCost::parse("{1}{R}"),
            ManaSpending::ANY_COLOR,
            [0; 6],
            Some((Red, 1).into()),
            [0; 6],
            credits,
        )
        .is_none()
    );
    let paid = payment_with_generic_credits(
        &pool(&[(Red, 1, true), (Red, 1, false)]),
        &ManaCost::parse("{2}{S}"),
        ManaSpending::EXACT,
        [0; 6],
        Some((Red, 1).into()),
        [0; 6],
        credits,
    )
    .unwrap();
    assert!(paid.remaining.is_empty());
    assert_eq!(paid.credits[Blue.index()], 1);
}

#[test]
fn ordinary_generic_assignment_matches_the_previous_exact_preference_order() {
    let mut seed = 0x619e_743a_u32;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        seed
    };
    for _ in 0..20_000 {
        let mut original = ManaPool::new();
        let mut required = [0; 6];
        let mut preferred = [0; 6];
        for color in ManaColor::ALL {
            original.add(color, next() % 5);
            original.add_snow(color, next() % 5);
            required[color.index()] = next() % 6;
            preferred[color.index()] = next() % 8;
        }
        let amount = next() % 32;
        let reference = previous_generic_payment(original.clone(), amount, required, preferred);
        let actual = payment_consuming(
            &original,
            &ManaCost::ZERO.with_more_generic(amount),
            ManaSpending::EXACT,
            preferred,
            None,
            required,
        );
        assert_eq!(actual, reference);
    }
}

// Independent reference preserves the previous in-place payment procedure,
// including ordinary-before-snow debiting and exact preference order.
fn previous_generic_payment(
    mut pool: ManaPool,
    mut amount: u32,
    required: [u32; 6],
    mut preferred: [u32; 6],
) -> Option<ManaPool> {
    for color in ManaColor::ALL {
        let need = required[color.index()];
        amount = amount.checked_sub(need)?;
        if !pool.spend(color, need) {
            return None;
        }
        preferred[color.index()] = preferred[color.index()].saturating_sub(need);
    }
    for color in ManaColor::ALL {
        let take = pool
            .available(color)
            .min(preferred[color.index()])
            .min(amount);
        pool.spend(color, take);
        amount -= take;
    }
    for color in ManaColor::ALL {
        let take = pool.available(color).min(amount);
        pool.spend(color, take);
        amount -= take;
    }
    (amount == 0).then_some(pool)
}
