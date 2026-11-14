use super::*;

#[test]
fn rank_expansion_is_a_bijection_for_every_small_exclusion_set() {
    for count in 0..=9 {
        for mask in 0..(1_u16 << count) {
            let excluded: Vec<u64> = (0..count).filter(|bit| mask & (1 << bit) != 0).collect();
            let expected: Vec<u64> = (0..count).filter(|slot| !excluded.contains(slot)).collect();
            let actual: Vec<_> = (0..expected.len() as u64)
                .map(|rank| expand(rank, &excluded))
                .collect();
            assert_eq!(actual, expected);
        }
    }
}

#[test]
fn unseen_preference_self_exclusion_and_empty_fallback_are_exact() {
    let seed = [7; 32];
    let mut population = Population {
        count: 4,
        generation: 12,
        own: Some(0),
        previous: &[1, 2, 2],
    };
    assert_eq!(
        select(&population, &seed).unwrap(),
        Some(Selection {
            generation: 12,
            slot: 3,
            repeated: false
        })
    );
    population.previous = &[3, 1, 2, 2];
    let repeated = select(&population, &seed).unwrap().unwrap();
    assert!(repeated.repeated);
    assert!((1..4).contains(&repeated.slot));
    population.previous = &[1, 2, 3];
    assert_eq!(select(&population, &seed).unwrap(), Some(repeated));
    population.count = 1;
    population.previous = &[];
    assert_eq!(select(&population, &seed).unwrap(), None);
    population.count = 0;
    population.own = None;
    assert_eq!(select(&population, &seed).unwrap(), None);
    population.own = Some(0);
    assert!(select(&population, &seed).is_err());
}

#[test]
fn choices_cover_the_global_index_and_reconnect_reuses_the_same_selection() {
    let population = Population {
        count: 1_000_000,
        generation: 34,
        own: Some(123),
        previous: &[300, 999_999],
    };
    let mut global = 0;
    for byte in 0..16 {
        let chosen = select(&population, &[byte; 32]).unwrap().unwrap();
        assert_eq!(select(&population, &[byte; 32]).unwrap(), Some(chosen));
        assert!(chosen.slot < population.count);
        assert!(![123, 300, 999_999].contains(&chosen.slot));
        global += usize::from(chosen.slot >= 512);
    }
    assert!(global > 0, "selection must not truncate to the first page");
    let largest = Population {
        count: i64::MAX as u64,
        generation: 0,
        own: None,
        previous: &[],
    };
    assert!(select(&largest, &[0; 32]).unwrap().unwrap().slot < largest.count);
    let invalid = Population {
        count: u64::MAX,
        ..largest
    };
    assert!(select(&invalid, &[0; 32]).is_err());
}

#[test]
fn withdrawal_keeps_every_surviving_account_in_exactly_one_dense_slot() {
    for count in 1..20 {
        for removed in 0..count {
            let plan = removal(count, removed).unwrap();
            let mut accounts: Vec<_> = (0..count).collect();
            if let Some(from) = plan.move_from {
                accounts[plan.removed as usize] = accounts[from as usize];
            }
            accounts.pop();
            assert_eq!(accounts.len() as u64, plan.remaining);
            assert!(!accounts.contains(&removed));
            accounts.sort_unstable();
            assert_eq!(
                accounts,
                (0..count)
                    .filter(|account| *account != removed)
                    .collect::<Vec<_>>()
            );
        }
    }
    assert!(removal(0, 0).is_err());
    assert!(removal(1, 1).is_err());
}
