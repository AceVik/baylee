use super::{Recipe, Sigil, Vessel};

impl Recipe {
    /// Pair-aware tuning prevents a single free activation from replenishing
    /// its entire own charge cost. Multi-item engines remain intentional.
    pub const fn level(self, tier: usize) -> Option<Level> {
        let Some(mut level) = self.vessel.level(tier) else {
            return None;
        };
        if tier == 3 {
            match (self.sigil, self.vessel) {
                (Sigil::Bloom, Vessel::Seed) => level.charges = 7,
                (Sigil::Lore | Sigil::Echo, Vessel::Lens) => level.charges = 3,
                _ => {}
            }
        }
        Some(level)
    }
}

/// Exact generation-six costs/rewards shared by native definitions and presentation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Level {
    /// Charges per matching event.
    pub gathered: u16,
    /// Charge counters removed as a cost, before any effects happen.
    pub charges: u16,
    /// Generic mana additionally paid.
    pub mana: u16,
    /// Effect magnitude (draw/loot/mana stay at one).
    pub amount: u16,
    /// Activations per turn; zero means no cap, only on Mythic.
    pub activations: u8,
}

impl Vessel {
    /// A zero-based rarity has one exact balance entry; unknown tiers refuse.
    pub const fn level(self, tier: usize) -> Option<Level> {
        if tier >= 4 {
            return None;
        }
        let (charges, mana, amount) = match self {
            Self::Chalice => ([3, 3, 3, 3], [0; 4], [2, 3, 4, 6]),
            Self::Scepter | Self::Bulwark => ([4, 4, 4, 4], [0; 4], [1, 2, 3, 4]),
            Self::Prism => ([3, 3, 3, 3], [0; 4], [1, 2, 3, 4]),
            Self::Seed | Self::Banner => ([5, 4, 4, 3], [0; 4], [1, 1, 2, 3]),
            Self::Tome => ([6, 5, 4, 3], [2, 2, 1, 1], [1; 4]),
            Self::Horn => ([6, 5, 4, 3], [1; 4], [1, 1, 2, 3]),
            Self::Flask => ([4, 3, 2, 2], [0; 4], [1; 4]),
            Self::Lens => ([4, 3, 3, 2], [0; 4], [1; 4]),
        };
        Some(Level {
            gathered: [1, 1, 2, 2][tier],
            charges: charges[tier],
            mana: mana[tier],
            amount: amount[tier],
            activations: [1, 1, 2, 0][tier],
        })
    }
    /// Human-readable original reward clause for native/LLM text generation.
    pub fn reward(self, amount: u16) -> String {
        match self {
            Self::Chalice => format!("You gain {amount} life"),
            Self::Scepter => format!("Each opponent loses {amount} life"),
            Self::Prism => format!("Scry {amount}"),
            Self::Seed => format!("Create {amount} 1/1 green Plant creature tokens"),
            Self::Tome => "Draw a card".into(),
            Self::Horn => format!("Create {amount} 1/1 white Soldier creature tokens"),
            Self::Flask => "Add one mana of any color".into(),
            Self::Bulwark => format!("Creatures you control get +0/+{amount} until end of turn"),
            Self::Banner => format!("Creatures you control get +{amount}/+0 until end of turn"),
            Self::Lens => "Draw a card, then discard a card".into(),
        }
    }
    /// Ability gained at Rare; retained by Mythic.
    pub const fn bonus_text(self) -> &'static str {
        match self {
            Self::Chalice | Self::Tome => "Whenever you draw a card, you gain 1 life.",
            Self::Scepter => {
                "Whenever one or more creatures you control attack, each opponent loses 1 life."
            }
            Self::Prism => "At the beginning of your upkeep, scry 1.",
            Self::Seed => "Creature tokens you control have vigilance.",
            Self::Horn => "Creature tokens you control get +0/+1.",
            Self::Flask => "Whenever you cast a noncreature spell, you gain 1 life.",
            Self::Bulwark => "Creatures you control have vigilance.",
            Self::Banner => "Creature tokens you control have haste.",
            Self::Lens => "Whenever you discard a card, you gain 1 life.",
        }
    }
}
