//! Public, structured offers for effect-granted special actions.
use baylee_cards_dsl::{SpecialActionCost, SpecialActionTiming};
use baylee_core::ids::{AbilityRef, DamageSourceRef, GrantedActionId, TargetRef};
use baylee_core::mana::ManaColor;

/// The immediate result, with every recipient already bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum GrantedActionKind {
    /// Add mana without using the stack.
    AddMana {
        /// Mana produced.
        color: ManaColor,
        /// Number of units.
        amount: u16,
    },
    /// Create a new finite shield on the exact original recipient.
    PreventNextDamage {
        /// Previously chosen recipient, never retargeted.
        target: TargetRef,
        /// Damage prevented by this use.
        amount: u32,
    },
}

/// One legal special action. Its ID is never reused after expiry.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct GrantedActionOffer {
    /// Identity to return, independent of vector order and object handles.
    pub id: GrantedActionId,
    /// Source incarnation that originally granted this action.
    pub source: DamageSourceRef,
    /// Printed rules provenance, even after that source has left.
    pub ability: Option<AbilityRef>,
    /// Timing permission; not an activated ability.
    pub timing: SpecialActionTiming,
    /// Explicit payment required on every use.
    pub cost: SpecialActionCost,
    /// Immediate result of a successful payment.
    pub effect: GrantedActionKind,
}
