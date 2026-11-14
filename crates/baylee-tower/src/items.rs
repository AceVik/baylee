//! Small initial catalog. Prices are calibration hypotheses, not measured balance claims.
use crate::Refusal;
use serde::{Deserialize, Serialize};

/// Maximum number of distinct relics equipped in a run.
pub const RELIC_SLOTS: usize = 3;
/// Consumables carried between games; charges are spent by verified engine records.
pub const POTION_SLOTS: u8 = 2;

/// Permanent-for-this-run equipment. Every effect is explicit and bounded.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Relic {
    /// Four additional starting life per rank, at most three ranks.
    Heartwood,
    /// One additional opening card and one additional maximum hand size.
    Satchel,
    /// One additional free London mulligan per rank, at most two ranks.
    Compass,
    /// One ordinary 1/1 green creature token at the beginning of the duel.
    Seedling,
}

impl Relic {
    /// Stable catalog order; never depends on a hash collection's iteration.
    pub const ALL: [Self; 4] = [
        Self::Heartwood,
        Self::Satchel,
        Self::Compass,
        Self::Seedling,
    ];

    /// Highest legal rank, bounding both economy and game execution.
    pub const fn max_rank(self) -> u8 {
        match self {
            Self::Heartwood => 3,
            Self::Compass => 2,
            Self::Satchel | Self::Seedling => 1,
        }
    }

    /// Authoritative purchase price for this next rank, not the cumulative value.
    pub const fn price(self, rank: u8) -> Option<u16> {
        match (self, rank) {
            (Self::Heartwood, 1) => Some(6),
            (Self::Heartwood, 2) | (Self::Seedling, 1) => Some(9),
            (Self::Heartwood, 3) | (Self::Compass, 1) => Some(12),
            (Self::Compass, 2) => Some(16),
            (Self::Satchel, 1) => Some(14),
            _ => None,
        }
    }

    /// Half the paid rank prices, rounded down once; never creates a resale loop.
    /// # Errors
    /// A zero or unsupported rank.
    pub fn sale(self, rank: u8) -> Result<u16, Refusal> {
        if rank == 0 || rank > self.max_rank() {
            return Err(Refusal::Invalid);
        }
        (1..=rank)
            .try_fold(0, |sum, rank| self.price(rank).map(|price| sum + price))
            .map(|total| total / 2)
            .ok_or(Refusal::Invalid)
    }
}

/// One-shot, ordinary engine choices. The client never edits life or mana itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Potion {
    /// Gain five life when legal; does not restore an expedition defeat allowance.
    Healing,
    /// Add one mana of a chosen color through an actual legal engine choice.
    Mana,
}

impl Potion {
    /// Catalog price, independent of a caller-supplied number.
    pub const fn price(self) -> u16 {
        match self {
            Self::Healing => 3,
            Self::Mana => 5,
        }
    }

    pub(crate) const fn index(self) -> usize {
        match self {
            Self::Healing => 0,
            Self::Mana => 1,
        }
    }
}

/// One equipped relic; improving its rank does not consume another slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Equipment {
    /// Catalog identity.
    pub relic: Relic,
    /// Purchased rank, validated before any effect or economy calculation.
    pub rank: u8,
}

/// Per-seat opening attributes. These are public facts, not opening-hand identities.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Attributes {
    /// Life when a fresh duel begins.
    pub life: u8,
    /// Number of cards drawn for each opening-hand decision.
    pub opening_cards: u8,
    /// Ordinary maximum hand size before other game effects.
    pub maximum_hand: u8,
    /// Opening redraws without a card disadvantage.
    pub free_mulligans: u8,
    /// Ordinary 1/1 starting tokens.
    pub seedlings: u8,
}

impl Default for Attributes {
    fn default() -> Self {
        Self {
            life: 20,
            opening_cards: 7,
            maximum_hand: 7,
            free_mulligans: 1,
            seedlings: 0,
        }
    }
}

/// Check slots, unique equipment kinds and ranks before projecting any effect.
/// # Errors
/// An oversized, repeated or impossible equipment loadout.
pub fn attributes(equipment: &[Equipment]) -> Result<Attributes, Refusal> {
    if equipment.len() > RELIC_SLOTS {
        return Err(Refusal::Capacity);
    }
    let mut result = Attributes::default();
    for (index, item) in equipment.iter().enumerate() {
        if item.rank == 0
            || item.rank > item.relic.max_rank()
            || equipment[..index]
                .iter()
                .any(|prior| prior.relic == item.relic)
        {
            return Err(Refusal::Invalid);
        }
        match item.relic {
            Relic::Heartwood => result.life += 4 * item.rank,
            Relic::Satchel => {
                result.opening_cards += 1;
                result.maximum_hand += 1;
            }
            Relic::Compass => result.free_mulligans += item.rank,
            Relic::Seedling => result.seedlings += 1,
        }
    }
    Ok(result)
}
