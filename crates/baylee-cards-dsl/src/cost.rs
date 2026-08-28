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
    /// Exile the source.
    ExileSelf,
}

/// A complete cost: a mana part plus non-mana parts.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Cost {
    /// The mana part (`ManaCost::ZERO` for none).
    pub mana: ManaCost,
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
