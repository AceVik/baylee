//! Compact financial invariants; the service also retains transactional entries.
use super::{DUEL_INCOME, Economy, Refusal, STARTING_GOLD};
use crate::v2::catalog::Potion;
use serde::{Deserialize, Serialize};

/// Monotonic run-local spending and legitimate sale proceeds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ledger {
    /// Purchases and paid refreshes, never client-submitted amounts.
    pub spent: u64,
    /// Proceeds of consumed owned instances, bounded by their inherited basis.
    pub resold: u64,
}

impl Ledger {
    pub(super) fn traded(self, before: u16, after: u16) -> Result<Self, Refusal> {
        let mut next = self;
        if after > before {
            next.resold = next
                .resold
                .checked_add(u64::from(after - before))
                .ok_or(Refusal::Capacity)?;
        } else {
            next.spent = next
                .spent
                .checked_add(u64::from(before - after))
                .ok_or(Refusal::Capacity)?;
        }
        Ok(next)
    }

    pub(super) fn validate(self, economy: &Economy, games: usize) -> Result<(), Refusal> {
        let earned = u64::from(STARTING_GOLD)
            + u64::from(DUEL_INCOME) * u64::try_from(games).map_err(|_| Refusal::Invalid)?;
        let wallet = earned
            .checked_add(self.resold)
            .and_then(|amount| amount.checked_sub(self.spent));
        let basis = economy
            .inventory
            .relics
            .iter()
            .map(|relic| u64::from(relic.basis))
            .sum::<u64>()
            + economy
                .inventory
                .materials
                .iter()
                .map(|material| u64::from(material.basis))
                .sum::<u64>()
            + economy
                .potions
                .iter()
                .zip([Potion::Healing, Potion::Mana])
                .map(|(count, potion)| u64::from(*count) * u64::from(potion.price()))
                .sum::<u64>();
        if wallet != Some(u64::from(economy.gold))
            || self.resold > self.spent / 2
            || u64::from(economy.gold) + basis > earned
        {
            return Err(Refusal::Invalid);
        }
        Ok(())
    }
}
