//! Atomic automatic upgrades. Only the newly bought lineage can cascade.
use super::{Definition, Economy, Layout, Recipe, Refusal, Relic, Slot};
use crate::v2::catalog::Rarity;

impl Economy {
    /// Buy one admitted offer and combine triples across equipment and storage.
    /// The caller must consume that exact offer in the same saved transaction.
    /// Gold, ingredients and equipment placement are unchanged on failure.
    /// # Errors
    /// Legacy inventory, unaffordable item, exhausted IDs or invalid final state.
    pub fn bought_automatic(&self, definition: Definition) -> Result<Self, Refusal> {
        self.bought_automatic_into(definition, None)
    }

    /// Buy/fuse and equip atomically. A fusion keeps its first equipped slot;
    /// otherwise the requested active slot must be free. Never displace gear.
    /// # Errors
    /// Invalid/occupied destination or any ordinary purchase/fusion refusal.
    pub fn bought_automatic_into(
        &self,
        definition: Definition,
        equip: Option<u8>,
    ) -> Result<Self, Refusal> {
        if self.inventory.layout != Layout::Run {
            return Err(Refusal::Unavailable);
        }
        if equip.is_some_and(|slot| slot >= self.inventory.layout.active_slots()) {
            return Err(Refusal::Invalid);
        }
        self.changed(|next| {
            let mut current = next.purchase(definition, Slot::Inventory)?;
            // Each iteration strictly increases Common -> Uncommon -> Rare.
            // Mythic always needs a deliberate targeted crafting-potion recipe.
            loop {
                let purchased = next
                    .inventory
                    .relics
                    .iter()
                    .find(|item| item.id == current)
                    .ok_or(Refusal::Invalid)?;
                if !matches!(
                    purchased.definition.rarity,
                    Rarity::Common | Rarity::Uncommon
                ) {
                    break;
                }
                let definition = purchased.definition;
                let first = next
                    .inventory
                    .relics
                    .iter()
                    .filter(|item| item.id != current && item.definition == definition)
                    .min_by_key(|item| ingredient_key(item));
                let Some(first) = first else { break };
                let second = next
                    .inventory
                    .relics
                    .iter()
                    .filter(|item| {
                        item.id != current && item.id != first.id && item.definition == definition
                    })
                    .min_by_key(|item| ingredient_key(item));
                let Some(second) = second else { break };
                let destination = [purchased, first, second]
                    .into_iter()
                    .filter_map(|item| match item.slot {
                        Slot::Active(index) => Some(index),
                        Slot::Inventory | Slot::Reserve(_) => None,
                    })
                    .min()
                    .map_or(Slot::Inventory, Slot::Active);
                let recipe = Recipe {
                    relics: vec![current, first.id, second.id],
                    mythic: None,
                    destination,
                };
                next.fuse(&recipe)?;
                current = next.inventory.relics.last().ok_or(Refusal::Invalid)?.id;
            }
            if let Some(slot) = equip {
                let item = next
                    .inventory
                    .relics
                    .iter()
                    .find(|item| item.id == current)
                    .ok_or(Refusal::Invalid)?;
                if item.slot == Slot::Inventory {
                    if next
                        .inventory
                        .relics
                        .iter()
                        .any(|item| item.slot == Slot::Active(slot))
                    {
                        return Err(Refusal::Capacity);
                    }
                    next.inventory
                        .relics
                        .iter_mut()
                        .find(|item| item.id == current)
                        .ok_or(Refusal::Invalid)?
                        .slot = Slot::Active(slot);
                }
            }
            Ok(())
        })
    }

    /// Right-click has one unambiguous meaning and never displaces another item.
    /// # Errors
    /// Legacy inventory, missing item, all six active slots occupied, or invalid state.
    pub fn toggled(&self, id: u32) -> Result<Self, Refusal> {
        self.validate()?;
        if self.inventory.layout != Layout::Run {
            return Err(Refusal::Unavailable);
        }
        let item = self
            .inventory
            .relics
            .iter()
            .find(|item| item.id == id)
            .ok_or(Refusal::Unavailable)?;
        let slot = match item.slot {
            Slot::Active(_) => Slot::Inventory,
            Slot::Inventory => (0..self.inventory.layout.active_slots())
                .find(|slot| {
                    !self
                        .inventory
                        .relics
                        .iter()
                        .any(|item| item.slot == Slot::Active(*slot))
                })
                .map(Slot::Active)
                .ok_or(Refusal::Capacity)?,
            Slot::Reserve(_) => return Err(Refusal::Invalid),
        };
        self.moved(id, slot)
    }
}

fn ingredient_key(item: &Relic) -> (u8, u32) {
    let slot = match item.slot {
        Slot::Active(index) => index,
        Slot::Inventory | Slot::Reserve(_) => u8::MAX,
    };
    (slot, item.id)
}

#[cfg(test)]
mod tests;
