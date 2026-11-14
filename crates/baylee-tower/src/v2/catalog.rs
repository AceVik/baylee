//! Frozen identities and balance hypotheses for the first native vertical slice.
use crate::Refusal;
use serde::{Deserialize, Serialize};

/// Relics contributing rules at a duel; reserve slots contribute nothing.
pub const ACTIVE_SLOTS: u8 = 3;
/// Inactive copies available for future builds and fusion.
pub const RESERVE_SLOTS: u8 = 6;
/// Full-price crafting material; never a table activation.
pub const MYTHIC_PRICE: u16 = 24;

/// Ordinary one-shot charges, separate from the Mythic crafting material.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Potion {
    /// Gain five life through an ordinary activated ability.
    Healing,
    /// Add one mana of the chosen color through an ordinary mana ability.
    Mana,
}

impl Potion {
    /// Frozen price of one charge in this revision.
    pub const fn price(self) -> u16 {
        match self {
            Self::Healing => 3,
            Self::Mana => 5,
        }
    }

    pub(super) const fn index(self) -> usize {
        match self {
            Self::Healing => 0,
            Self::Mana => 1,
        }
    }
}

/// Stable upgrade tiers; the final tier needs a Rare and one crafting potion.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rarity {
    /// Three identical copies fuse into Uncommon.
    Common,
    /// Three identical copies fuse into Rare.
    Uncommon,
    /// One copy and a Mythic Potion fuse into Mythic Rare.
    Rare,
    /// Final tier, not sold directly.
    MythicRare,
}

impl Rarity {
    /// Stable order for catalog presentation and definition encoding.
    pub const ALL: [Self; 4] = [Self::Common, Self::Uncommon, Self::Rare, Self::MythicRare];

    /// One-based tier used by fixed native definitions.
    pub const fn tier(self) -> u8 {
        match self {
            Self::Common => 1,
            Self::Uncommon => 2,
            Self::Rare => 3,
            Self::MythicRare => 4,
        }
    }

    /// Fixed purchase price; Mythic equipment is crafted only.
    pub const fn price(self) -> Option<u16> {
        match self {
            Self::Common => Some(4),
            Self::Uncommon => Some(10),
            Self::Rare => Some(26),
            Self::MythicRare => None,
        }
    }

    /// Greatest paid basis reachable through this revision's legitimate recipes.
    pub const fn maximum_basis(self) -> u16 {
        match self {
            Self::Common => 4,
            Self::Uncommon => 12,
            Self::Rare => 36,
            Self::MythicRare => 60,
        }
    }

    /// Upper resale bound, in addition to half the actual inherited paid basis.
    pub const fn sale_cap(self) -> u16 {
        match self {
            Self::Common => 2,
            Self::Uncommon => 5,
            Self::Rare => 13,
            Self::MythicRare => 25,
        }
    }
}

/// Original equipment families in the first native pack.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Family {
    /// Additional starting life; no persistent injury between duels.
    Heartwood,
    /// Additional opening cards and maximum hand size.
    Satchel,
    /// Additional free London mulligans.
    Compass,
    /// A removable starting artifact that grows Plants at its controller's upkeep.
    Grove,
}

impl Family {
    /// Frozen catalog order, independent of map iteration or translated names.
    pub const ALL: [Self; 4] = [Self::Heartwood, Self::Satchel, Self::Compass, Self::Grove];

    const fn offset(self) -> u16 {
        match self {
            Self::Heartwood => 0,
            Self::Satchel => 4,
            Self::Compass => 8,
            Self::Grove => 12,
        }
    }
}

/// An immutable family/tier pair in revision two; never contains caller effects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    /// Family shared by all copies in a fusion recipe.
    pub family: Family,
    /// Exact rarity, not a freely chosen numeric rank.
    pub rarity: Rarity,
}

impl Definition {
    /// Stable nonzero ID in a native manifest. Never reorder or reuse these IDs.
    pub const fn id(self) -> u16 {
        self.family.offset() + self.rarity.tier() as u16
    }

    /// Resolve only this revision's supported IDs.
    pub fn from_id(id: u16) -> Option<Self> {
        let zero = id.checked_sub(1)?;
        Some(Self {
            family: *Family::ALL.get(usize::from(zero / 4))?,
            rarity: Rarity::ALL[usize::from(zero % 4)],
        })
    }
}

/// Exactly derived public starting attributes, shared with native pack admission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Attributes {
    /// Fresh duel life.
    pub life: i32,
    /// Cards per opening-hand decision.
    pub cards: u8,
    /// Base cleanup hand limit.
    pub maximum_hand: u8,
    /// Free London redraws.
    pub free_mulligans: u8,
}

/// Validate the complete active loadout without silently clamping bought effects.
/// # Errors
/// Too many active relics or a combined opening resource outside engine bounds.
pub fn attributes(active: &[Definition]) -> Result<Attributes, Refusal> {
    if active.len() > usize::from(ACTIVE_SLOTS) {
        return Err(Refusal::Capacity);
    }
    let mut result = Attributes {
        life: 20,
        cards: 7,
        maximum_hand: 7,
        free_mulligans: 1,
    };
    for item in active {
        let tier = usize::from(item.rarity.tier() - 1);
        match item.family {
            Family::Heartwood => result.life += [4, 7, 11, 16][tier],
            Family::Satchel => {
                result.cards += [0, 1, 1, 2][tier];
                result.maximum_hand += [1, 1, 2, 2][tier];
            }
            Family::Compass => result.free_mulligans += item.rarity.tier(),
            Family::Grove => {}
        }
    }
    if result.cards > 9 || result.maximum_hand > 9 || result.free_mulligans > 7 {
        return Err(Refusal::Capacity);
    }
    Ok(result)
}
