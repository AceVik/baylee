//! Bounded run income from equipment frozen before the verified duel.
use super::{Outcome, Refusal};
use crate::v2::catalog::{Definition, Family, RUN_ACTIVE_SLOTS};
use serde::{Deserialize, Serialize};

/// Saved equipped Acorn counts by rarity. No gold amount comes from a player.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Income {
    /// Common, Uncommon, Rare, Mythic Rare counts at reservation time.
    pub acorns: [u8; 4],
}

impl Income {
    pub(super) fn freeze(active: &[Definition]) -> Result<Self, Refusal> {
        if active.len() > usize::from(RUN_ACTIVE_SLOTS) {
            return Err(Refusal::Invalid);
        }
        let mut income = Self::default();
        for item in active
            .iter()
            .filter(|item| item.family == Family::GildedAcorn)
        {
            income.acorns[usize::from(item.rarity.tier() - 1)] += 1;
        }
        Ok(income)
    }

    /// Compute the exact bonus: 1/2/3/4 per Acorn, plus 3 per Mythic on victory.
    /// This is bounded by six slots and independent of life totals or loop count.
    /// # Errors
    /// A persisted count exceeding the fixed equipment capacity.
    pub fn bonus(self, outcome: Outcome) -> Result<u16, Refusal> {
        if self
            .acorns
            .iter()
            .map(|count| u16::from(*count))
            .sum::<u16>()
            > u16::from(RUN_ACTIVE_SLOTS)
        {
            return Err(Refusal::Invalid);
        }
        let base: u16 = self
            .acorns
            .iter()
            .zip([1, 2, 3, 4])
            .map(|(count, value)| u16::from(*count) * value)
            .sum();
        Ok(base
            + if outcome == Outcome::Victory {
                u16::from(self.acorns[3]) * 3
            } else {
                0
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn six_mythics_are_bounded_and_cannot_mint_unlimited_combo_income() {
        let income = Income {
            acorns: [0, 0, 0, 6],
        };
        assert_eq!(income.bonus(Outcome::Victory), Ok(42));
        assert_eq!(income.bonus(Outcome::Defeat), Ok(24));
        assert_eq!(income.bonus(Outcome::Draw), Ok(24));
        assert!(
            Income {
                acorns: [1, 0, 0, 6]
            }
            .bonus(Outcome::Victory)
            .is_err()
        );
    }
}
