//! Bounded owned instances. IDs are monotonic ordinals within one run, not UUIDs.
use super::catalog::{self, Definition};
use crate::Refusal;
use serde::{Deserialize, Serialize};

/// A visible, explicit destination; reserve equipment never affects the duel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "index",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Slot {
    /// Zero-based active equipment slot.
    Active(u8),
    /// Zero-based inactive storage slot.
    Reserve(u8),
}

impl Slot {
    pub(super) const fn valid(self) -> bool {
        match self {
            Self::Active(index) => index < catalog::ACTIVE_SLOTS,
            Self::Reserve(index) => index < catalog::RESERVE_SLOTS,
        }
    }
}

/// One specific owned relic, including its conserved paid basis.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Relic {
    /// Run-local instance identity; never reused after sale or fusion.
    pub id: u32,
    /// Immutable catalog definition.
    pub definition: Definition,
    /// Equipped or stored position.
    pub slot: Slot,
    /// Gold actually spent on all surviving ancestors; a free grant has zero.
    pub basis: u16,
}

/// One Mythic crafting potion, separate from the two table consumable charges.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Material {
    /// Uses the same run-local instance sequence as relics.
    pub id: u32,
    /// Actual gold basis, at most the catalog price.
    pub basis: u16,
}

/// Authoritative inventory, validated after loading and before every transition.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Inventory {
    /// Next unused ordinal. Exhaustion refuses the operation, never wraps.
    pub next_id: u32,
    /// Canonical ascending instance order, at most nine relics.
    pub relics: Vec<Relic>,
    /// Canonical ascending instance order, at most two crafting potions.
    pub materials: Vec<Material>,
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            next_id: 1,
            relics: Vec::new(),
            materials: Vec::new(),
        }
    }
}

impl Inventory {
    /// Reject identity reuse, overlapping slots, forged basis and invalid loadouts.
    /// # Errors
    /// Malformed persisted state or an impossible combined active loadout.
    pub fn validate(&self) -> Result<(), Refusal> {
        if self.next_id == 0 || self.relics.len() > 9 || self.materials.len() > 2 {
            return Err(Refusal::Invalid);
        }
        for (index, relic) in self.relics.iter().enumerate() {
            if relic.id == 0
                || relic.id >= self.next_id
                || !relic.slot.valid()
                || relic.basis > relic.definition.rarity.maximum_basis()
                || (index > 0 && self.relics[index - 1].id >= relic.id)
                || self.relics[..index]
                    .iter()
                    .any(|other| other.slot == relic.slot)
            {
                return Err(Refusal::Invalid);
            }
        }
        for (index, material) in self.materials.iter().enumerate() {
            if material.id == 0
                || material.id >= self.next_id
                || material.basis > catalog::MYTHIC_PRICE
                || (index > 0 && self.materials[index - 1].id >= material.id)
                || self.relics.iter().any(|relic| relic.id == material.id)
            {
                return Err(Refusal::Invalid);
            }
        }
        catalog::attributes(&self.active())?;
        Ok(())
    }

    /// Active definitions in slot order, suitable for a frozen native manifest.
    /// Call `validate` before using deserialized state.
    pub fn active(&self) -> Vec<Definition> {
        (0..catalog::ACTIVE_SLOTS)
            .filter_map(|index| {
                self.relics
                    .iter()
                    .find(|relic| relic.slot == Slot::Active(index))
                    .map(|relic| relic.definition)
            })
            .collect()
    }

    pub(super) fn allocate(&mut self) -> Result<u32, Refusal> {
        let id = self.next_id;
        self.next_id = id.checked_add(1).ok_or(Refusal::Capacity)?;
        Ok(id)
    }
}
