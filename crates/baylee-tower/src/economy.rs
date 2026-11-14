//! Atomic pure economy transitions. Persistence supplies the run lock and request deduplication.
use crate::{
    MAX_GOLD, Refusal, STARTING_GOLD,
    items::{self, Equipment, Potion, Relic},
};
use serde::{Deserialize, Serialize};

/// Bounded wallet and non-card inventory of one run. Call `validate` after decoding.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Economy {
    /// Spendable run currency, never persistent account power.
    pub gold: u16,
    /// Equipment currently carried, at most three distinct relics.
    pub equipment: Vec<Equipment>,
    /// Healing and mana charges, in that stable order; at most two in total.
    pub potions: [u8; 2],
}

impl Default for Economy {
    fn default() -> Self {
        Self {
            gold: STARTING_GOLD,
            equipment: Vec::new(),
            potions: [0; 2],
        }
    }
}

/// A purchaseable catalog object. Price and availability come from the saved shop.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Merchandise {
    /// Buy exactly the next equipment rank; a stale rank cannot buy another.
    Relic {
        /// Equipment kind.
        relic: Relic,
        /// Offered next rank.
        rank: u8,
    },
    /// One consumable charge.
    Potion {
        /// Consumable kind.
        potion: Potion,
    },
}

impl Merchandise {
    /// Resolve price without trusting a price supplied by a player.
    /// # Errors
    /// A rank not defined in this catalog revision.
    pub const fn price(self) -> Result<u16, Refusal> {
        match self {
            Self::Relic { relic, rank } => match relic.price(rank) {
                Some(price) => Ok(price),
                None => Err(Refusal::Invalid),
            },
            Self::Potion { potion } => Ok(potion.price()),
        }
    }
}

impl Economy {
    /// Reject malformed serialized state before any mutation or projection.
    /// # Errors
    /// Wallet, slot, charge or equipment-rank bounds are exceeded.
    pub fn validate(&self) -> Result<(), Refusal> {
        if self.gold > MAX_GOLD
            || self.potions.iter().map(|n| u16::from(*n)).sum::<u16>()
                > u16::from(items::POTION_SLOTS)
        {
            return Err(Refusal::Invalid);
        }
        items::attributes(&self.equipment)?;
        Ok(())
    }

    /// Apply an offer already checked against the authority's current shop.
    /// A refusal leaves the original value untouched, including the wallet.
    /// # Errors
    /// Malformed state/offer, stale rank, full inventory or insufficient gold.
    pub fn bought(&self, offer: Merchandise) -> Result<Self, Refusal> {
        self.validate()?;
        let price = offer.price()?;
        let gold = self.gold.checked_sub(price).ok_or(Refusal::Gold)?;
        let mut next = self.clone();
        match offer {
            Merchandise::Relic { relic, rank } => {
                if let Some(owned) = next.equipment.iter_mut().find(|item| item.relic == relic) {
                    if rank != owned.rank + 1 {
                        return Err(Refusal::Stale);
                    }
                    owned.rank = rank;
                } else {
                    if rank != 1 {
                        return Err(Refusal::Stale);
                    }
                    if next.equipment.len() == items::RELIC_SLOTS {
                        return Err(Refusal::Capacity);
                    }
                    next.equipment.push(Equipment { relic, rank });
                    next.equipment.sort_by_key(|item| item.relic);
                }
            }
            Merchandise::Potion { potion } => {
                if next.potions.iter().sum::<u8>() == items::POTION_SLOTS {
                    return Err(Refusal::Capacity);
                }
                next.potions[potion.index()] += 1;
            }
        }
        next.gold = gold;
        next.validate()?;
        Ok(next)
    }

    /// Sell an entire equipped relic for half its total purchase cost.
    /// # Errors
    /// Malformed state, unowned equipment or a wallet that cannot hold the sale.
    pub fn sold(&self, relic: Relic) -> Result<Self, Refusal> {
        self.validate()?;
        let index = self
            .equipment
            .iter()
            .position(|item| item.relic == relic)
            .ok_or(Refusal::Unavailable)?;
        let value = relic.sale(self.equipment[index].rank)?;
        let mut next = self.clone();
        next.gold = next
            .gold
            .checked_add(value)
            .filter(|gold| *gold <= MAX_GOLD)
            .ok_or(Refusal::Capacity)?;
        next.equipment.remove(index);
        Ok(next)
    }

    /// Consume exactly the charges verified in the authoritative terminal record.
    /// A technical void does not call this transition.
    /// # Errors
    /// Malformed state or a record claiming more charges than were reserved.
    pub fn consumed(&self, used: [u8; 2]) -> Result<Self, Refusal> {
        self.validate()?;
        let mut next = self.clone();
        for (remaining, used) in next.potions.iter_mut().zip(used) {
            *remaining = remaining.checked_sub(used).ok_or(Refusal::Invalid)?;
        }
        Ok(next)
    }
}

#[cfg(test)]
mod tests;
