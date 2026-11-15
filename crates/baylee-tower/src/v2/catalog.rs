//! Stable identities and balance hypotheses for generation-bound native catalogs.
use crate::Refusal;
use serde::{Deserialize, Serialize};

/// Relics contributing rules at a duel; reserve slots contribute nothing.
pub const ACTIVE_SLOTS: u8 = 3;
/// Inactive copies available for future builds and fusion.
pub const RESERVE_SLOTS: u8 = 6;
/// Independent equipment slots in the redesigned run contract.
pub const RUN_ACTIVE_SLOTS: u8 = 6;
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

/// Original equipment families; a run's generation admits its frozen subset.
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
    /// A removable artifact that gains life at its controller's upkeep.
    SpringChalice,
    /// A removable artifact that gains life when its controller's creatures enter.
    Hearthstone,
    /// Life-gain events store charges for a paid damage activation.
    BloodAmber,
    /// Mana and life buy a card; only Mythic loses the tap cost.
    LifeCoil,
    /// Life-gain events offer an optional mana payment for Plants.
    SeedCradle,
    /// A removable artifact that supports only your creature tokens.
    SwarmBanner,
    /// Dying creature tokens store charges for colorless mana.
    HarvestWheel,
    /// Entering creature tokens store charges for green mana.
    RootReservoir,
    /// Noncreature spells drain opposing life.
    EmberLantern,
    /// Draws store charges for card selection.
    TideglassLens,
    /// Your nontoken creature deaths drain opposing life.
    GraveBell,
    /// Noncreature spells store charges for mana.
    StormSpindle,
    /// Toughness and keyword protection for your creatures.
    DawnAegis,
    /// Bounded additional income after verified duels.
    GildedAcorn,
}

impl Family {
    /// Frozen catalog order, independent of map iteration or translated names.
    pub const ALL: [Self; 18] = [
        Self::Heartwood,
        Self::Satchel,
        Self::Compass,
        Self::Grove,
        Self::SpringChalice,
        Self::Hearthstone,
        Self::BloodAmber,
        Self::LifeCoil,
        Self::SeedCradle,
        Self::SwarmBanner,
        Self::HarvestWheel,
        Self::RootReservoir,
        Self::EmberLantern,
        Self::TideglassLens,
        Self::GraveBell,
        Self::StormSpindle,
        Self::DawnAegis,
        Self::GildedAcorn,
    ];

    const fn offset(self) -> u16 {
        match self {
            Self::Heartwood => 0,
            Self::Satchel => 4,
            Self::Compass => 8,
            Self::Grove => 12,
            Self::SpringChalice => 16,
            Self::Hearthstone => 20,
            Self::BloodAmber => 24,
            Self::LifeCoil => 28,
            Self::SeedCradle => 32,
            Self::SwarmBanner => 36,
            Self::HarvestWheel => 40,
            Self::RootReservoir => 44,
            Self::EmberLantern => 48,
            Self::TideglassLens => 52,
            Self::GraveBell => 56,
            Self::StormSpindle => 60,
            Self::DawnAegis => 64,
            Self::GildedAcorn => 68,
        }
    }
}

/// An immutable family/tier pair in Tower v2; never contains caller effects.
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
    derive_attributes(active, ACTIVE_SLOTS, 9, 7)
}

/// Six-slot run attributes; all bonuses apply without hidden clamping.
/// # Errors
/// More than six equipped items or an unsupported derived starting resource.
pub fn run_attributes(active: &[Definition]) -> Result<Attributes, Refusal> {
    derive_attributes(active, RUN_ACTIVE_SLOTS, 32, 32)
}

fn derive_attributes(
    active: &[Definition],
    slots: u8,
    hand_limit: u8,
    mulligan_limit: u8,
) -> Result<Attributes, Refusal> {
    if active.len() > usize::from(slots) {
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
                if slots == RUN_ACTIVE_SLOTS {
                    result.cards += [0, 1, 2, 3][tier];
                    result.maximum_hand += [1, 2, 3, 4][tier];
                } else {
                    result.cards += [0, 1, 1, 2][tier];
                    result.maximum_hand += [1, 1, 2, 2][tier];
                }
            }
            Family::Compass => result.free_mulligans += item.rarity.tier(),
            Family::Grove
            | Family::SpringChalice
            | Family::Hearthstone
            | Family::BloodAmber
            | Family::LifeCoil
            | Family::SeedCradle
            | Family::SwarmBanner
            | Family::HarvestWheel
            | Family::RootReservoir
            | Family::EmberLantern
            | Family::TideglassLens
            | Family::GraveBell
            | Family::StormSpindle
            | Family::DawnAegis
            | Family::GildedAcorn => {}
        }
    }
    if result.cards > hand_limit
        || result.maximum_hand > hand_limit
        || result.free_mulligans > mulligan_limit
    {
        return Err(Refusal::Capacity);
    }
    Ok(result)
}
