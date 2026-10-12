//! Bounded allocation for ordinary mana and generic-only cost credits.
//! Normal symbols are assigned first; this kernel solves the remaining generic
//! amount jointly, without enumerating units or assigning a tax greedily first.

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Resources {
    pub available: [u32; 6],
    pub required: [u32; 6],
}

#[derive(Debug)]
pub(super) struct Allocation {
    pub ordinary: [u32; 6],
    pub restricted: [u32; 6],
}

fn sum(units: &[u32; 6]) -> u64 {
    units.iter().map(|&n| u64::from(n)).sum()
}

pub(super) fn allocate(
    ordinary: Resources,
    restricted: Resources,
    amount: u32,
    quota: u32,
    preferred: [u32; 6],
) -> Option<Allocation> {
    if (0..6).any(|i| {
        ordinary.required[i] > ordinary.available[i]
            || restricted.required[i] > restricted.available[i]
    }) {
        return None;
    }
    let mandatory_restricted = sum(&restricted.required);
    let mandatory_ordinary = sum(&ordinary.required);
    if mandatory_restricted > u64::from(quota)
        || mandatory_restricted + mandatory_ordinary > u64::from(amount)
    {
        return None;
    }
    let mut result = Allocation {
        ordinary: ordinary.required,
        restricted: restricted.required,
    };
    let mut left = u64::from(amount) - mandatory_restricted - mandatory_ordinary;
    let mut allowance = (u64::from(quota) - mandatory_restricted).min(left);
    // All mandatory normal units are already reserved. Consuming the largest
    // allowed credit remainder can only reduce the normal generic demand.
    for i in 0..6 {
        let available = restricted.available[i] - result.restricted[i];
        let take = u32::try_from(u64::from(available).min(allowance))
            .expect("bounded by the available u32 amount");
        result.restricted[i] += take;
        allowance -= u64::from(take);
        left -= u64::from(take);
    }
    // Preserve the ordinary payer's exact preference order when no credits exist.
    for (i, preference) in preferred.into_iter().enumerate() {
        let available = ordinary.available[i] - result.ordinary[i];
        let wanted = preference.saturating_sub(ordinary.required[i]);
        let take = u32::try_from(u64::from(available.min(wanted)).min(left))
            .expect("bounded by the available u32 amount");
        result.ordinary[i] += take;
        left -= u64::from(take);
    }
    for i in 0..6 {
        let available = ordinary.available[i] - result.ordinary[i];
        let take = u32::try_from(u64::from(available).min(left))
            .expect("bounded by the available u32 amount");
        result.ordinary[i] += take;
        left -= u64::from(take);
    }
    (left == 0).then_some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_allocation_matches_exhaustive_credit_assignments() {
        let mut seed = 0x93a4_27f1_u32;
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed
        };
        for case in 0..20_000 {
            let mut normal = Resources::default();
            let mut credit = Resources::default();
            let a = case % 6;
            let b = (a + 1) % 6;
            for i in [a, b] {
                normal.available[i] = next() % 5;
                normal.required[i] = next() % 6;
                credit.available[i] = next() % 5;
                credit.required[i] = next() % 6;
            }
            let amount = next() % 17;
            let quota = next() % 9;
            let expected = normal.required[a] <= normal.available[a]
                && normal.required[b] <= normal.available[b]
                && (credit.required[a]..=credit.available[a]).any(|x| {
                    (credit.required[b]..=credit.available[b]).any(|y| {
                        x + y <= quota
                            && x + y <= amount
                            && sum(&normal.required) <= u64::from(amount - x - y)
                            && u64::from(amount - x - y) <= sum(&normal.available)
                    })
                });
            let result = allocate(normal, credit, amount, quota, [2; 6]);
            assert_eq!(result.is_some(), expected, "case {case}");
            if let Some(result) = result {
                assert_eq!(
                    sum(&result.ordinary) + sum(&result.restricted),
                    u64::from(amount)
                );
                assert!(sum(&result.restricted) <= u64::from(quota));
                for i in 0..6 {
                    assert!(
                        (normal.required[i]..=normal.available[i]).contains(&result.ordinary[i])
                    );
                    assert!(
                        (credit.required[i]..=credit.available[i]).contains(&result.restricted[i])
                    );
                }
            }
        }
    }

    #[test]
    fn maximum_cost_is_solved_without_unit_iteration_or_overflow() {
        let resources = Resources {
            available: [u32::MAX; 6],
            required: [0; 6],
        };
        let paid = allocate(resources, resources, u32::MAX, 2_000_000_000, [u32::MAX; 6])
            .expect("bounded by the available u32 amount");
        assert_eq!(sum(&paid.restricted), 2_000_000_000);
        assert_eq!(sum(&paid.ordinary), u64::from(u32::MAX) - 2_000_000_000);
    }
}
