//! Child of `mana_pay`. Required individual units distinguish snow and plain.
#[allow(clippy::wildcard_imports)]
use super::*;
#[derive(Clone, Copy)]
struct Unit {
    class: usize,
    color: ManaColor,
    snow: bool,
    required: bool,
}
#[derive(Clone, Copy)]
enum Pip {
    Snow,
    Color(ManaColor),
}
fn reference(
    units: &[Unit],
    pips: &[Pip],
    generic: u32,
    quota: u32,
    constrained: bool,
    used: u16,
) -> bool {
    if let Some((pip, rest)) = pips.split_first() {
        return units.iter().enumerate().any(|(i, unit)| {
            used & (1 << i) == 0
                && (unit.class == 0 || constrained)
                && match pip {
                    Pip::Snow => unit.snow,
                    Pip::Color(color) => unit.color == *color,
                }
                && reference(units, rest, generic, quota, constrained, used | (1 << i))
        });
    }
    for selected in 0_u16..(1_u16 << units.len()) {
        if selected & used != 0 || selected.count_ones() != generic {
            continue;
        }
        if units
            .iter()
            .enumerate()
            .filter(|(i, unit)| selected & (1 << i) != 0 && unit.class == 1)
            .count()
            > usize::try_from(quota).unwrap()
        {
            continue;
        }
        if units
            .iter()
            .enumerate()
            .all(|(i, unit)| !unit.required || (selected | used) & (1 << i) != 0)
        {
            return true;
        }
    }
    false
}
fn check(units: &[Unit], pips: &[Pip], cost: &str, generic: u32, quota: u32, constrained: bool) {
    let mut pools = [ManaPool::new(), ManaPool::new()];
    let mut receipts = [ReceiptRequirement::default(); 2];
    for unit in units {
        if unit.snow {
            pools[unit.class].add_snow(unit.color, 1);
        } else {
            pools[unit.class].add(unit.color, 1);
        }
        if unit.required {
            let required = if unit.snow {
                &mut receipts[unit.class].snow
            } else {
                &mut receipts[unit.class].plain
            };
            required[unit.color.index()] += 1;
        }
    }
    let before = pools.clone();
    let expected = reference(units, pips, generic, quota, constrained, 0);
    let paid = payment_with_source_receipts(
        &pools[0],
        &ManaCost::parse(cost),
        ManaSpending::EXACT,
        [0; 6],
        None,
        [0; 6],
        SourceCredits {
            pool: pools[1].clone(),
            required: [0; 6],
            generic_quota: quota,
            constrained,
        },
        receipts,
    );
    assert_eq!(
        paid.is_some(),
        expected,
        "cost={cost} quota={quota} constrained={constrained} receipts={receipts:?} pools={pools:?}"
    );
    assert_eq!(pools, before);
    if let Some(paid) = paid {
        let remaining = [paid.remaining, paid.restricted_remaining];
        assert_eq!(
            pools[0].total() + pools[1].total() - remaining[0].total() - remaining[1].total(),
            u64::try_from(pips.len()).unwrap() + u64::from(generic)
        );
        for class in 0..2 {
            for color in ManaColor::ALL {
                let snow_spent =
                    pools[class].snow_available(color) - remaining[class].snow_available(color);
                let plain_before =
                    pools[class].available(color) - pools[class].snow_available(color);
                let plain_after =
                    remaining[class].available(color) - remaining[class].snow_available(color);
                assert!(snow_spent >= receipts[class].snow[color.index()]);
                assert!(plain_before - plain_after >= receipts[class].plain[color.index()]);
            }
        }
    }
}
#[test]
fn exact_receipts_match_individual_unit_reference_for_all_small_resource_classes() {
    // Each of 8 categories is absent, optional or required: 3^8 scenarios.
    // No duplicate within category, so required identity is unambiguous.
    for encoded in 0_u32..3_u32.pow(8) {
        let mut code = encoded;
        let mut units = Vec::new();
        for class in 0..2 {
            for color in [ManaColor::Red, ManaColor::Blue] {
                for snow in [false, true] {
                    let state = code % 3;
                    code /= 3;
                    if state != 0 {
                        units.push(Unit {
                            class,
                            color,
                            snow,
                            required: state == 2,
                        });
                    }
                }
            }
        }
        let costs = [
            (vec![Pip::Snow], "{S}", 0),
            (vec![Pip::Snow, Pip::Color(ManaColor::Red)], "{S}{R}", 0),
            (vec![Pip::Snow], "{1}{S}", 1),
            (vec![Pip::Snow, Pip::Color(ManaColor::Blue)], "{1}{S}{U}", 1),
            (vec![Pip::Snow], "{2}{S}", 2),
        ];
        for (pips, cost, generic) in costs {
            for quota in 0..=2 {
                for constrained in [false, true] {
                    check(&units, &pips, cost, generic, quota, constrained);
                }
            }
        }
    }
}
#[test]
fn snow_color_total_cannot_stand_in_for_a_required_plain_unit() {
    let units = [
        Unit {
            class: 0,
            color: ManaColor::Red,
            snow: true,
            required: false,
        },
        Unit {
            class: 0,
            color: ManaColor::Red,
            snow: false,
            required: true,
        },
    ];
    // Exactly one S must consume snow, so it cannot also consume required plain.
    check(&units, &[Pip::Snow], "{S}", 0, 0, false);
    check(&units, &[Pip::Snow], "{1}{S}", 1, 0, false);
    let mut restricted = units;
    for unit in &mut restricted {
        unit.class = 1;
    }
    check(&restricted, &[Pip::Snow], "{S}", 0, 0, true);
    check(&restricted, &[Pip::Snow], "{1}{S}", 1, 1, true);
}
