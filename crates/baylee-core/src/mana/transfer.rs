//! Pool transfer preserves each unit's restrictions and provenance (CR 106.13).

use super::ManaPool;

impl ManaPool {
    /// The exact mana consumed between two snapshots of one payment.
    /// No production or unrelated pool mutation may occur between snapshots.
    #[must_use]
    pub fn payment_receipt(&self, after: &Self) -> Option<Self> {
        let mut paid = Self::new();
        for i in 0..6 {
            paid.plain[i] = self.plain[i].checked_sub(after.plain[i])?;
            paid.snow[i] = self.snow[i].checked_sub(after.snow[i])?;
        }
        paid.restricted = consumed_entries(&self.restricted, &after.restricted)?;
        paid.ridden = consumed_entries(&self.ridden, &after.ridden)?;
        Some(paid)
    }

    /// Transfer every unspent unit into another pool, preserving snow,
    /// retention flags, restrictions and spend riders.
    ///
    /// Returns `false` without changing either pool if a plain counter would
    /// exceed this engine's representable capacity. The caller handles the
    /// same-player case without borrowing a pool twice.
    pub fn transfer_to(&mut self, destination: &mut Self) -> bool {
        let mut plain = [0; 6];
        let mut snow = [0; 6];
        for i in 0..6 {
            let Some(total) = destination.plain[i].checked_add(self.plain[i]) else {
                return false;
            };
            let Some(snow_total) = destination.snow[i].checked_add(self.snow[i]) else {
                return false;
            };
            plain[i] = total;
            snow[i] = snow_total;
        }
        destination.plain = plain;
        destination.snow = snow;
        destination.restricted.append(&mut self.restricted);
        destination.ridden.append(&mut self.ridden);
        self.plain = [0; 6];
        self.snow = [0; 6];
        true
    }
}

fn consumed_entries(
    before: &[super::RestrictedMana],
    after: &[super::RestrictedMana],
) -> Option<Vec<super::RestrictedMana>> {
    let mut remaining = after.to_vec();
    let mut paid = Vec::new();
    for entry in before {
        let mut kept = entry.amount;
        for later in &mut remaining {
            if later.restriction == entry.restriction
                && later.color == entry.color
                && later.flags == entry.flags
            {
                let shared = kept.min(later.amount);
                kept -= shared;
                later.amount -= shared;
            }
        }
        if kept != 0 {
            paid.push(super::RestrictedMana {
                amount: kept,
                ..*entry
            });
        }
    }
    remaining
        .iter()
        .all(|entry| entry.amount == 0)
        .then_some(paid)
}

#[cfg(test)]
mod tests {
    use super::super::{ManaColor, ManaFlags, RestrictedMana, RestrictionId};
    use super::*;

    #[test]
    fn transfer_keeps_snow_restrictions_riders_and_retention_without_duplication() {
        let mut source = ManaPool::new();
        source.add(ManaColor::Blue, 2);
        source.add_snow(ManaColor::Blue, 1);
        let rider = RestrictedMana {
            color: ManaColor::Green,
            amount: 3,
            flags: ManaFlags::SNOW.union(ManaFlags::NO_EMPTY),
            restriction: RestrictionId(7),
        };
        source.add_ridden(rider);
        let restricted = RestrictedMana {
            color: ManaColor::Red,
            amount: 4,
            flags: ManaFlags::NO_EMPTY.union(ManaFlags::UNTIL_END_OF_TURN),
            restriction: RestrictionId(8),
        };
        source.add_restricted(restricted);
        let mut destination = ManaPool::new();
        destination.add(ManaColor::Colorless, 5);
        assert!(source.transfer_to(&mut destination));
        assert_eq!(source, ManaPool::new());
        assert_eq!(destination.total(), 15);
        assert_eq!(destination.available(ManaColor::Blue), 3);
        assert_eq!(destination.snow_available(ManaColor::Blue), 1);
        assert_eq!(destination.snow_available(ManaColor::Green), 3);
        assert_eq!(destination.ridden(), [rider]);
        assert_eq!(destination.restricted(), [restricted]);
        destination.empty_at_step_end();
        assert_eq!(destination.total(), 7);
        destination.expire_turn_retention();
        destination.empty_at_step_end();
        assert_eq!(destination.total(), 3);
        assert_eq!(destination.take_ridden_units(7, 2).unwrap().amount, 2);
        assert_eq!(destination.total(), 1);
    }

    #[test]
    fn transfer_capacity_refusal_keeps_both_complete_pools() {
        let mut source = ManaPool::new();
        source.add(ManaColor::White, 2);
        source.add_restricted(RestrictedMana {
            color: ManaColor::Black,
            amount: 3,
            flags: ManaFlags::SNOW,
            restriction: RestrictionId(41),
        });
        let mut destination = ManaPool::new();
        destination.add(ManaColor::White, u32::MAX - 1);
        let before = (source.clone(), destination.clone());
        assert!(!source.transfer_to(&mut destination));
        assert_eq!((source, destination), before);
    }

    #[test]
    fn transfer_and_receipt_preserve_more_than_u16_units_of_one_type() {
        let mut source = ManaPool::new();
        source.add_snow(ManaColor::Blue, 60_000);
        let mut destination = ManaPool::new();
        destination.add(ManaColor::Blue, 60_000);
        assert!(source.transfer_to(&mut destination));
        assert!(source.is_empty());
        assert_eq!(destination.available(ManaColor::Blue), 120_000);
        assert_eq!(destination.snow_available(ManaColor::Blue), 60_000);
        let before = destination.clone();
        assert!(destination.spend(ManaColor::Blue, 90_000));
        assert_eq!(destination.snow_available(ManaColor::Blue), 30_000);
        let paid = before.payment_receipt(&destination).unwrap();
        assert_eq!(paid.available(ManaColor::Blue), 90_000);
        assert_eq!(paid.snow_available(ManaColor::Blue), 30_000);
        let before = destination.clone();
        assert!(!destination.spend_snow_units(ManaColor::Blue, 30_001));
        assert_eq!(destination, before, "an oversized snow charge is atomic");
        assert!(destination.spend_snow_units(ManaColor::Blue, 30_000));
        assert!(destination.is_empty());
    }

    #[test]
    fn total_counts_full_counters_exactly_across_all_types() {
        let mut pool = ManaPool::new();
        for color in ManaColor::ALL {
            pool.add(color, u32::MAX);
        }
        pool.add_restricted(RestrictedMana {
            color: ManaColor::Green,
            amount: u16::MAX,
            flags: ManaFlags::NONE,
            restriction: RestrictionId(1),
        });
        assert_eq!(pool.total(), 6 * u64::from(u32::MAX) + u64::from(u16::MAX));
        assert!(!pool.is_empty());
    }

    #[test]
    fn checked_production_refuses_capacity_overflow_without_losing_any_units() {
        let mut pool = ManaPool::new();
        assert!(pool.try_add(ManaColor::Blue, u32::MAX - 1));
        assert!(pool.try_add_snow(ManaColor::Blue, 1));
        let before = pool.clone();
        assert!(!pool.try_add(ManaColor::Blue, 1));
        assert_eq!(pool, before);
        assert!(!pool.try_add_snow(ManaColor::Blue, 1));
        assert_eq!(pool, before);
        assert_eq!(pool.snow_available(ManaColor::Blue), 1);
        assert!(pool.spend_snow(ManaColor::Blue));
        assert!(pool.try_add_snow(ManaColor::Blue, 1));
        assert_eq!(pool, before);
    }

    #[test]
    fn payment_receipt_tracks_actual_types_snow_and_partial_restricted_spending() {
        let mut before = ManaPool::new();
        before.add(ManaColor::Colorless, 3);
        before.add_snow(ManaColor::White, 2);
        let entry = RestrictedMana {
            color: ManaColor::Blue,
            amount: 3,
            flags: ManaFlags::SNOW,
            restriction: RestrictionId(9),
        };
        before.add_restricted(entry);
        let mut after = before.clone();
        assert!(after.spend(ManaColor::Colorless, 1));
        assert!(after.spend_snow(ManaColor::White));
        assert_eq!(after.take_restricted_units(9, 2).unwrap().amount, 2);
        let paid = before.payment_receipt(&after).unwrap();
        assert_eq!(paid.total(), 4);
        assert_eq!(paid.available(ManaColor::Colorless), 1);
        assert_eq!(paid.available(ManaColor::White), 1);
        assert_eq!(paid.snow_available(ManaColor::White), 1);
        assert_eq!(paid.restricted(), [RestrictedMana { amount: 2, ..entry }]);
        assert!(after.payment_receipt(&before).is_none());
    }
}
