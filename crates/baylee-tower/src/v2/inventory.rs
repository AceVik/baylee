//! Bounded owned instances. IDs are monotonic ordinals within one run, not UUIDs.
use super::catalog::{self, Definition};
use crate::Refusal;
use serde::{Deserialize, Serialize};

/// Frozen storage contract; absent in the original saved generations.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Layout {
    /// The retained three-equipped/six-reserve contract.
    #[default]
    Legacy,
    /// Six equipped positions, with densely stored run inventory.
    Run,
}

impl Layout {
    #[allow(clippy::trivially_copy_pass_by_ref)] // serde's skip predicate
    const fn is_legacy(&self) -> bool {
        matches!(self, Self::Legacy)
    }

    /// Number of active item positions; never the amount of stored inventory.
    pub const fn active_slots(self) -> u8 {
        match self {
            Self::Legacy => catalog::ACTIVE_SLOTS,
            Self::Run => catalog::RUN_ACTIVE_SLOTS,
        }
    }
}

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
    /// Run inventory has no numbered empty slots. Items retain their own IDs.
    Inventory,
}

impl Slot {
    pub(super) const fn valid(self, layout: Layout) -> bool {
        match (self, layout) {
            (Self::Active(index), layout) => index < layout.active_slots(),
            (Self::Reserve(index), Layout::Legacy) => index < catalog::RESERVE_SLOTS,
            (Self::Inventory, Layout::Run) => true,
            _ => false,
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
    /// Kept out of legacy serialization so old commitments stay byte-identical.
    #[serde(default, skip_serializing_if = "Layout::is_legacy")]
    pub layout: Layout,
    /// Next unused ordinal. Exhaustion refuses the operation, never wraps.
    pub next_id: u32,
    /// Canonical ascending instance order; only occupied inventory uses memory.
    pub relics: Vec<Relic>,
    /// Canonical ascending instance order, at most two crafting potions.
    pub materials: Vec<Material>,
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            layout: Layout::Legacy,
            next_id: 1,
            relics: Vec::new(),
            materials: Vec::new(),
        }
    }
}

impl Inventory {
    /// Empty independent-run inventory. Nothing carries over from another run.
    #[must_use]
    pub fn for_run() -> Self {
        Self {
            layout: Layout::Run,
            ..Self::default()
        }
    }

    /// Reject identity reuse, overlapping slots, forged basis and invalid loadouts.
    /// # Errors
    /// Malformed persisted state or an impossible combined active loadout.
    pub fn validate(&self) -> Result<(), Refusal> {
        if self.next_id == 0
            || (self.layout == Layout::Legacy
                && (self.relics.len() > 9 || self.materials.len() > 2))
        {
            return Err(Refusal::Invalid);
        }
        let mut occupied = 0_u16;
        for (index, relic) in self.relics.iter().enumerate() {
            if relic.id == 0
                || relic.id >= self.next_id
                || !relic.slot.valid(self.layout)
                || relic.basis > relic.definition.maximum_basis()
                || (index > 0 && self.relics[index - 1].id >= relic.id)
            {
                return Err(Refusal::Invalid);
            }
            let position = match relic.slot {
                Slot::Active(index) => Some(index),
                Slot::Reserve(index) => Some(catalog::ACTIVE_SLOTS + index),
                Slot::Inventory => None,
            };
            if let Some(position) = position {
                let bit = 1_u16 << position;
                if occupied & bit != 0 {
                    return Err(Refusal::Invalid);
                }
                occupied |= bit;
            }
        }
        for (index, material) in self.materials.iter().enumerate() {
            if material.id == 0
                || material.id >= self.next_id
                || material.basis > catalog::MYTHIC_PRICE
                || (index > 0 && self.materials[index - 1].id >= material.id)
                || self
                    .relics
                    .binary_search_by_key(&material.id, |relic| relic.id)
                    .is_ok()
            {
                return Err(Refusal::Invalid);
            }
        }
        match self.layout {
            Layout::Legacy => catalog::attributes(&self.active())?,
            Layout::Run => catalog::run_attributes(&self.active())?,
        };
        Ok(())
    }

    /// Active definitions in slot order, suitable for a frozen native manifest.
    /// Call `validate` before using deserialized state.
    pub fn active(&self) -> Vec<Definition> {
        (0..self.layout.active_slots())
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
