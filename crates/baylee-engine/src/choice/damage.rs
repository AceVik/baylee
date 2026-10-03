//! Public decisions while a simultaneous damage event is being modified.

use super::{AnswerFault, ObjectId, PlayerId};
use crate::event::DamageTarget;
use baylee_core::ids::AbilityRef;

/// Identity of one decision within one damage event. An earlier answer may
/// not answer a later question, even if it happens to name the same effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct DamageChoiceId {
    /// Monotonically assigned damage event identity.
    pub batch: u64,
    /// Monotonically assigned decision within that event.
    pub step: u64,
}

/// One source's damage to one recipient, before it is dealt.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct DamagePartView {
    /// Stable part identity within the damage event; not a vector offset.
    pub id: u32,
    /// The object dealing the damage.
    pub source: ObjectId,
    /// Where this damage would currently be dealt.
    pub recipient: DamageTarget,
    /// Damage still due, and the greatest allocatable prevention share.
    pub amount: u32,
    /// Whether this is combat damage.
    pub is_combat: bool,
    /// False when prevention cannot reduce this damage. Other consequences,
    /// such as removing a counter, can still apply (CR 615.12).
    pub preventable: bool,
}

/// What applying an offered damage replacement/prevention effect does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum DamageEffectKind {
    /// Spend a finite shield; excess capacity remains for later damage.
    PreventNext {
        /// Shield capacity before applying this effect.
        remaining: u32,
    },
    /// Prevention bought for this event only; excess capacity expires.
    PreventThisEvent {
        /// Remaining prevention for this event.
        remaining: u32,
    },
    /// Prevent all the indicated combat damage without spending the effect.
    PreventCombat,
    /// Protection prevents damage from this source.
    Protection,
    /// A chosen-source shield is consumed by the next matching instance.
    PreventFromSource {
        /// Damage source the shield names.
        source: ObjectId,
        /// Damage left unprevented, as on Forcefield.
        all_but: u32,
        /// Its controller gains life equal to the amount prevented.
        gain_life: bool,
    },
    /// Deal the indicated damage to another recipient instead.
    Redirect {
        /// The new recipient.
        to: DamageTarget,
    },
    /// Redirect a finite amount, allocating it among simultaneous sources.
    RedirectNext {
        /// Remaining total redirection capacity.
        remaining: u32,
        /// The new recipient.
        to: DamageTarget,
    },
    /// Remove one counter and try to prevent one damage. The effect can
    /// apply again to another point, so other effects may be chosen between.
    RemoveCounter {
        /// The counter consumed.
        kind: baylee_cards_dsl::CounterKind,
        /// Counters currently available.
        remaining: u32,
    },
}

/// A legal effect, with sufficient provenance and affected damage to label
/// the choice without guessing from a card's name or Oracle text.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct DamageEffectOption {
    /// Stable option identity within this event, never a display offset.
    pub id: u32,
    /// The permanent or spell that created this effect, where available.
    pub source: Option<ObjectId>,
    /// The originating printed ability, where available.
    pub ability: Option<AbilityRef>,
    /// Controller of the effect (also the recipient of its life gain).
    pub controller: PlayerId,
    /// What selecting this effect does.
    pub kind: DamageEffectKind,
    /// Part identities modified by this option.
    pub parts: Vec<u32>,
}

/// Validate a complete prevention allocation. Omitted parts get zero;
/// explicit zero shares are legal. No part may occur twice.
pub(super) fn allocation_fault(
    parts: &[DamagePartView],
    total: u32,
    allocation: &[(u32, u32)],
) -> Option<AnswerFault> {
    let mut sum = 0_u64;
    for (at, &(id, amount)) in allocation.iter().enumerate() {
        if allocation[..at].iter().any(|&(old, _)| old == id) {
            return Some(AnswerFault::Repeated);
        }
        let Some(part) = parts.iter().find(|part| part.id == id) else {
            return Some(AnswerFault::NotOffered);
        };
        if amount > part.amount {
            return Some(AnswerFault::OutOfRange);
        }
        sum += u64::from(amount);
    }
    match sum.cmp(&u64::from(total)) {
        std::cmp::Ordering::Less => Some(AnswerFault::TotalTooLow),
        std::cmp::Ordering::Greater => Some(AnswerFault::TotalTooHigh),
        std::cmp::Ordering::Equal => None,
    }
}
