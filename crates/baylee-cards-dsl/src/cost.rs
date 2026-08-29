//! Ability costs: mana plus non-mana parts (see `docs/cost-model.md`).

use crate::filter::Filter;
use baylee_core::mana::ManaCost;

/// A non-mana cost part (tap, sacrifice, life, …).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum CostPart {
    /// Tap the source (`{T}`).
    TapSelf,
    /// Untap the source (`{Q}`).
    UntapSelf,
    /// Sacrifice the source.
    SacrificeSelf,
    /// Sacrifice a permanent matching the filter.
    Sacrifice(&'static Filter),
    /// Pay life.
    PayLife(u16),
    /// Discard a card matching the filter (choice at payment).
    Discard(&'static Filter),
    /// Discard the source card itself (cycling).
    DiscardSelf,
    /// Exile the source.
    ExileSelf,
    /// Return the source to its owner's hand (Recurring Nightmare).
    ReturnSelfToHand,
    /// Exile a card from your hand matching the filter (pitch costs).
    ExileFromHand(&'static Filter),
    /// Pay life equal to the spell's X value (Toxic Deluge).
    PayLifeX,
}

/// A conditional cost reduction printed on a card (Surgical Metamorph).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum CostReduction {
    /// Costs {N} less if you weren't the starting player.
    NotStartingPlayer(u32),
}

/// When an alternative cost may be used.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum AltCondition {
    /// Always (Force of Will, evoke).
    Always,
    /// Only when it is not your turn (Force of Negation).
    NotYourTurn,
    /// Only while you control your commander (Fierce Guardianship).
    CommanderControlled,
}

/// An alternative way to pay a spell's cost (CR 601.2b).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct AlternativeCost {
    /// What you pay instead of the mana cost.
    pub cost: Cost,
    /// When it may be used.
    pub condition: AltCondition,
}

/// A complete cost: a mana part plus non-mana parts.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Cost {
    /// The mana part (`ManaCost::ZERO` for none).
    pub mana: baylee_core::mana::ManaCost,
    /// The non-mana parts.
    pub parts: &'static [CostPart],
}

impl Cost {
    /// Free.
    pub const FREE: Cost = Cost {
        mana: ManaCost::ZERO,
        parts: &[],
    };

    /// Tap-only (`{T}: …`).
    pub const TAP: Cost = Cost {
        mana: ManaCost::ZERO,
        parts: &[CostPart::TapSelf],
    };
}
