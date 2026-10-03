//! Temporary special actions created by resolving spells and abilities.
use crate::{Amount, TargetSpec};
use baylee_core::mana::{ManaColor, ManaCost};

/// When a granted special action may be taken (CR 116.2c).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum SpecialActionTiming {
    /// Whenever its player has priority, including under split second.
    Priority,
    /// Whenever its player could activate a mana ability, including payments.
    ManaAbility,
}

/// A payment made for a special action, never an activation cost.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum SpecialActionCost {
    /// Pay this much life; zero life cannot pay a positive amount.
    Life(u32),
    /// Pay unrestricted mana as an ordinary non-spell, non-ability cost.
    Mana(ManaCost),
}

/// The immediate result of using a granted special action.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SpecialActionEffect {
    /// Add a fixed quantity of mana.
    AddMana {
        /// Mana produced.
        color: ManaColor,
        /// Number of units.
        amount: u16,
    },
    /// Prevent damage to the original recipient, bound when the grant resolves.
    PreventNextDamage {
        /// Recipient of every use, not chosen again.
        target: TargetSpec,
        /// Shield created by each use.
        amount: Amount,
    },
}
