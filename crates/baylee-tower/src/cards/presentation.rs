//! Frozen presentation facts: the same edition/finish follows a card into loot.
use baylee_core::preset::{Finish, PrintInfo};
use serde::{Deserialize, Serialize};

/// Reference-print rarity, independently reviewed against the native registry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardRarity {
    /// One base ticket.
    Common,
    /// Two base tickets.
    Uncommon,
    /// Three base tickets.
    Rare,
    /// Four base tickets.
    Mythic,
}

impl CardRarity {
    /// Small integer weights keep sampling bounded and floating-point free.
    pub const fn tickets(self) -> u8 {
        match self {
            Self::Common => 1,
            Self::Uncommon => 2,
            Self::Rare => 3,
            Self::Mythic => 4,
        }
    }
}

/// Authority-published cosmetics; no URLs, scripts or player-selected identities.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Presentation {
    /// Exact English printing; clients can resolve the same edition in their language.
    pub printing: uuid::Uuid,
    /// Cosmetic finish used in both the frozen native preset and reward previews.
    pub finish: Finish,
    /// Reviewed reference rarity, used for loot weight and its visual border.
    pub rarity: CardRarity,
}

impl Presentation {
    /// Freeze the fallback printing into the native preset. Language-specific
    /// art may change presentation only, never rules identity or reward ownership.
    pub fn print(self) -> PrintInfo {
        PrintInfo {
            scryfall_id: self.printing,
            lang: "en".to_owned(),
            finish: self.finish,
        }
    }
}
