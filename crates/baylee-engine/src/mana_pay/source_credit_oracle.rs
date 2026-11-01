//! Child module of `mana_pay`. Independent tiny unit-assignment oracle.
#[allow(clippy::wildcard_imports)]
use super::*;
#[derive(Clone, Copy)]
struct Unit {
    class: usize,
    color: ManaColor,
    snow: bool,
}
#[derive(Clone, Copy)]
enum Pip {
    Color(ManaColor),
    Snow,
}
struct Problem<'a> {
    units: &'a [Unit],
    pips: &'a [Pip],
    generic: u32,
    quota: u32,
    constrained: bool,
    wild: bool,
    required: [[u32; 6]; 2],
    restriction: Option<(ManaColor, u32)>,
}
fn oracle(p: &Problem<'_>, position: usize, used: u16) -> bool {
    if let Some(pip) = p.pips.get(position) {
        return p.units.iter().enumerate().any(|(i, unit)| {
            used & (1 << i) == 0
                && (unit.class == 0 || p.constrained)
                && match pip {
                    Pip::Snow => unit.snow,
                    Pip::Color(color) => {
                        unit.color == *color || (p.wild && *color != ManaColor::Colorless)
                    }
                }
                && oracle(p, position + 1, used | (1 << i))
        });
    }
    // Enumerate every subset of remaining actual units for generic demand.
    for subset in 0_u16..(1_u16 << p.units.len()) {
        if subset & used != 0 || subset.count_ones() != p.generic {
            continue;
        }
        let mut generic_restricted = 0;
        let mut fixed = 0;
        let mut consumed = [[0_u32; 6]; 2];
        for (i, unit) in p.units.iter().enumerate() {
            if subset & (1 << i) != 0 {
                generic_restricted += u32::from(unit.class == 1);
                fixed += u32::from(p.restriction.is_some_and(|(color, _)| unit.color == color));
            }
            if (subset | used) & (1 << i) != 0 {
                consumed[unit.class][unit.color.index()] += 1;
            }
        }
        if generic_restricted <= p.quota
            && p.restriction.is_none_or(|(_, amount)| fixed >= amount)
            && (0..2)
                .all(|class| (0..6).all(|color| consumed[class][color] >= p.required[class][color]))
        {
            return true;
        }
    }
    false
}
fn resources(units: &[Unit], class: usize) -> ManaPool {
    let mut pool = ManaPool::new();
    for unit in units.iter().filter(|unit| unit.class == class) {
        if unit.snow {
            pool.add_snow(unit.color, 1);
        } else {
            pool.add(unit.color, 1);
        }
    }
    pool
}
#[allow(clippy::too_many_arguments)] // Explicit independent oracle dimensions mirror the solver contract.
fn compare(
    units: &[Unit],
    pips: &[Pip],
    cost: &str,
    generic: u32,
    quota: u32,
    constrained: bool,
    wild: bool,
    required: [[u32; 6]; 2],
    restriction: Option<(ManaColor, u32)>,
) {
    let ordinary = resources(units, 0);
    let restricted = resources(units, 1);
    let ordinary_before = ordinary.clone();
    let restricted_before = restricted.clone();
    let expected = oracle(
        &Problem {
            units,
            pips,
            generic,
            quota,
            constrained,
            wild,
            required,
            restriction,
        },
        0,
        0,
    );
    let paid = payment_with_source_credits(
        &ordinary,
        &ManaCost::parse(cost),
        if wild {
            ManaSpending::ANY_COLOR
        } else {
            ManaSpending::EXACT
        },
        [0; 6],
        restriction.map(Into::into),
        required[0],
        SourceCredits {
            pool: restricted.clone(),
            required: required[1],
            generic_quota: quota,
            constrained,
        },
    );
    assert_eq!(
        paid.is_some(),
        expected,
        "cost={cost} quota={quota} constrained={constrained} wild={wild} required={required:?} restriction={restriction:?}"
    );
    if !constrained && restriction.is_none() {
        let specialized = payment_with_generic_credits(
            &ordinary,
            &ManaCost::parse(cost),
            if wild {
                ManaSpending::ANY_COLOR
            } else {
                ManaSpending::EXACT
            },
            [0; 6],
            None,
            required[0],
            GenericCredits {
                available: ManaColor::ALL.map(|color| restricted.available(color)),
                required: required[1],
                quota,
            },
        );
        assert_eq!(
            paid.is_some(),
            specialized.is_some(),
            "generic-only specialization remains exact"
        );
    }
    assert_eq!(ordinary, ordinary_before);
    assert_eq!(restricted, restricted_before);
    if let Some(paid) = paid {
        assert_eq!(
            ordinary.total() + restricted.total()
                - paid.remaining.total()
                - paid.restricted_remaining.total(),
            u64::try_from(pips.len()).unwrap() + u64::from(generic)
        );
        for color in ManaColor::ALL {
            assert!(paid.remaining.available(color) <= ordinary.available(color));
            assert!(paid.restricted_remaining.available(color) <= restricted.available(color));
            assert!(
                ordinary.available(color) - paid.remaining.available(color)
                    >= required[0][color.index()]
            );
            assert!(
                restricted.available(color) - paid.restricted_remaining.available(color)
                    >= required[1][color.index()]
            );
        }
    }
}
#[test]
fn source_credits_match_exhaustive_two_color_reference_assignments() {
    for counts in 0_u32..81 {
        let mut value = counts;
        let mut units = Vec::new();
        for (class, color) in [
            (0, ManaColor::Red),
            (0, ManaColor::Blue),
            (1, ManaColor::Red),
            (1, ManaColor::Blue),
        ] {
            for _ in 0..value % 3 {
                units.push(Unit {
                    class,
                    color,
                    snow: false,
                });
            }
            value /= 3;
        }
        for red in 0..=2 {
            for blue in 0..=2 {
                for generic in 0..=3 {
                    for quota in 0..=3 {
                        for constrained in [false, true] {
                            for obligation in 0..4 {
                                let mut required = [[0; 6]; 2];
                                required[0][ManaColor::Red.index()] =
                                    u32::from(obligation & 1 != 0);
                                required[1][ManaColor::Blue.index()] =
                                    u32::from(obligation & 2 != 0);
                                let mut pips = vec![Pip::Color(ManaColor::Red); red];
                                pips.extend(vec![Pip::Color(ManaColor::Blue); blue]);
                                let cost = format!(
                                    "{{{generic}}}{}{}",
                                    "{R}".repeat(red),
                                    "{U}".repeat(blue)
                                );
                                compare(
                                    &units,
                                    &pips,
                                    &cost,
                                    generic,
                                    quota,
                                    constrained,
                                    false,
                                    required,
                                    None,
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}
#[test]
fn source_credits_preserve_snow_wild_color_and_literal_colorless_constraints() {
    for restricted_snow in [false, true] {
        for constrained in [false, true] {
            for wild in [false, true] {
                let units = [
                    Unit {
                        class: 0,
                        color: ManaColor::Colorless,
                        snow: false,
                    },
                    Unit {
                        class: 1,
                        color: ManaColor::Blue,
                        snow: restricted_snow,
                    },
                ];
                for (pips, cost, generic) in [
                    (vec![Pip::Snow], "{1}{S}", 1),
                    (vec![Pip::Color(ManaColor::Colorless)], "{1}{C}", 1),
                    (
                        vec![Pip::Color(ManaColor::Red), Pip::Color(ManaColor::Colorless)],
                        "{R}{C}",
                        0,
                    ),
                ] {
                    compare(
                        &units,
                        &pips,
                        cost,
                        generic,
                        0,
                        constrained,
                        wild,
                        [[0; 6]; 2],
                        None,
                    );
                }
            }
        }
    }
}
#[test]
fn source_credits_fixed_generic_color_is_actual_type_and_respects_eligible_quota() {
    let units = [
        Unit {
            class: 0,
            color: ManaColor::Blue,
            snow: false,
        },
        Unit {
            class: 1,
            color: ManaColor::Red,
            snow: false,
        },
    ];
    for quota in [0, 1] {
        for constrained in [false, true] {
            for color in [ManaColor::Red, ManaColor::Blue] {
                compare(
                    &units,
                    &[Pip::Color(ManaColor::Blue)],
                    "{1}{U}",
                    1,
                    quota,
                    constrained,
                    true,
                    [[0; 6]; 2],
                    Some((color, 1)),
                );
            }
        }
    }
}
