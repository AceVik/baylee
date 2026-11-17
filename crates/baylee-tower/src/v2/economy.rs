//! Atomic owned-state transitions. Service authorization and shop stock are separate.
use super::{
    STARTING_GOLD,
    catalog::{Definition, MYTHIC_PRICE, Potion},
    inventory::{Inventory, Layout, Material, Relic, Slot},
};
use crate::{MAX_GOLD, Refusal};
use serde::{Deserialize, Serialize};

mod automatic;
mod fusion;
pub use fusion::Recipe;

/// Currency, equipment and charges of a revision-two run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Economy {
    /// Run-local gold; never paid interest or transferred to another run.
    pub gold: u16,
    /// Owned equipment and crafting materials.
    pub inventory: Inventory,
    /// Healing and mana charges, in that order; two combined.
    pub potions: [u8; 2],
}

impl Default for Economy {
    fn default() -> Self {
        Self {
            gold: STARTING_GOLD,
            inventory: Inventory::default(),
            potions: [0; 2],
        }
    }
}

impl Economy {
    /// Fresh run economy; no account or earlier-run item state is accepted.
    #[must_use]
    pub fn for_run() -> Self {
        Self {
            inventory: Inventory::for_run(),
            ..Self::default()
        }
    }

    /// Check persisted state before any price calculation or mutation.
    /// # Errors
    /// Malformed inventory or wallet/charge bounds.
    pub fn validate(&self) -> Result<(), Refusal> {
        if self.gold > self.gold_limit()
            || self.potions.iter().map(|n| u16::from(*n)).sum::<u16>() > 2
        {
            return Err(Refusal::Invalid);
        }
        self.inventory.validate()
    }

    /// Generation-four's maximum is below 1024 even with six Mythics every duel;
    /// older wallets retain their exact bound of 200.
    const fn gold_limit(&self) -> u16 {
        match self.inventory.layout {
            Layout::Legacy => MAX_GOLD,
            Layout::Run => 1024,
        }
    }

    fn changed(
        &self,
        action: impl FnOnce(&mut Self) -> Result<(), Refusal>,
    ) -> Result<Self, Refusal> {
        self.validate()?;
        let mut next = self.clone();
        action(&mut next)?;
        next.validate()?;
        Ok(next)
    }

    fn purchase(&mut self, definition: Definition, slot: Slot) -> Result<u32, Refusal> {
        let price = definition.price().ok_or(Refusal::Unavailable)?;
        self.gold = self.gold.checked_sub(price).ok_or(Refusal::Gold)?;
        let id = self.inventory.allocate()?;
        self.inventory.relics.push(Relic {
            id,
            definition,
            slot,
            basis: price,
        });
        Ok(id)
    }

    /// Buy a relic from an offer already admitted by the locked authoritative shop.
    /// # Errors
    /// Insufficient gold, occupied/illegal destination or unavailable rarity.
    pub fn bought(&self, definition: Definition, slot: Slot) -> Result<Self, Refusal> {
        self.changed(|next| next.purchase(definition, slot).map(|_| ()))
    }

    /// Buy a crafting potion, after shop availability is checked by the authority.
    /// # Errors
    /// Insufficient gold, two existing materials or exhausted identity sequence.
    pub fn bought_material(&self) -> Result<Self, Refusal> {
        self.changed(|next| {
            if next.inventory.materials.len() == 2 {
                return Err(Refusal::Capacity);
            }
            next.gold = next.gold.checked_sub(MYTHIC_PRICE).ok_or(Refusal::Gold)?;
            let id = next.inventory.allocate()?;
            next.inventory.materials.push(Material {
                id,
                basis: MYTHIC_PRICE,
            });
            Ok(())
        })
    }

    /// Buy one table charge from an admitted authoritative shop offer.
    /// # Errors
    /// Two charges already owned or insufficient gold.
    pub fn bought_potion(&self, potion: Potion) -> Result<Self, Refusal> {
        self.changed(|next| {
            if next.potions.iter().sum::<u8>() == 2 {
                return Err(Refusal::Capacity);
            }
            next.gold = next.gold.checked_sub(potion.price()).ok_or(Refusal::Gold)?;
            next.potions[potion.index()] += 1;
            Ok(())
        })
    }

    /// Settle charges observed in a verified terminal record and grant equal income.
    /// The run transition must bind the result and advance its phase exactly once;
    /// technical interruptions never call this operation.
    /// # Errors
    /// More charges claimed than reserved, invalid state or wallet overflow.
    pub fn settled(&self, used: [u8; 2]) -> Result<Self, Refusal> {
        self.changed(|next| {
            for (remaining, spent) in next.potions.iter_mut().zip(used) {
                *remaining = remaining.checked_sub(spent).ok_or(Refusal::Invalid)?;
            }
            next.gold = next
                .gold
                .checked_add(super::DUEL_INCOME)
                .filter(|gold| *gold <= next.gold_limit())
                .ok_or(Refusal::Capacity)?;
            Ok(())
        })
    }

    /// Move equipment; an occupied destination swaps with its current slot.
    /// # Errors
    /// Unknown instance, illegal slot or combined active resources beyond bounds.
    pub fn moved(&self, id: u32, slot: Slot) -> Result<Self, Refusal> {
        self.changed(|next| {
            if !slot.valid(next.inventory.layout) {
                return Err(Refusal::Invalid);
            }
            let index = next
                .inventory
                .relics
                .iter()
                .position(|item| item.id == id)
                .ok_or(Refusal::Unavailable)?;
            let previous = next.inventory.relics[index].slot;
            if let Some(other) = next
                .inventory
                .relics
                .iter_mut()
                .find(|item| slot != Slot::Inventory && item.slot == slot)
            {
                other.slot = previous;
            }
            next.inventory.relics[index].slot = slot;
            Ok(())
        })
    }

    /// Sell exactly one owned instance. Free grants never return gold.
    /// # Errors
    /// Unknown instance or a wallet unable to hold the full legitimate sale.
    pub fn sold(&self, id: u32) -> Result<Self, Refusal> {
        self.changed(|next| {
            let price =
                if let Some(index) = next.inventory.relics.iter().position(|item| item.id == id) {
                    let relic = next.inventory.relics.remove(index);
                    (relic.basis / 2).min(relic.definition.rarity.sale_cap())
                } else {
                    let index = next
                        .inventory
                        .materials
                        .iter()
                        .position(|item| item.id == id)
                        .ok_or(Refusal::Unavailable)?;
                    next.inventory.materials.remove(index).basis / 2
                };
            next.gold = next
                .gold
                .checked_add(price)
                .filter(|gold| *gold <= next.gold_limit())
                .ok_or(Refusal::Capacity)?;
            Ok(())
        })
    }

    /// Credit authoritative proceeds from selling one unused nonbasic run card.
    /// # Errors
    /// Legacy layout, zero gold or a wallet unable to hold the full legitimate sale.
    pub fn sold_card(&self, gold: u16) -> Result<Self, Refusal> {
        self.changed(|next| {
            if next.inventory.layout != Layout::Run || gold == 0 {
                return Err(Refusal::Unavailable);
            }
            next.gold = next
                .gold
                .checked_add(gold)
                .filter(|total| *total <= next.gold_limit())
                .ok_or(Refusal::Capacity)?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests;
