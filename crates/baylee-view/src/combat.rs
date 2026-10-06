use baylee_core::ids::{Defender, ObjectId};
use serde::{Deserialize, Serialize};

// -------------------------------------------------------------------- combat

/// One declared attacker and what it attacks.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct AttackerView {
    /// The attacking creature.
    pub creature: ObjectId,
    /// What it attacks: the defending player, or one of their
    /// planeswalkers (CR 506.2).
    pub defending: Defender,
    /// Whether it was blocked, which is **not** the same question as whether
    /// anything is blocking it now (CR 509.1h).
    ///
    /// A creature that was blocked stays blocked for the rest of combat even
    /// if every blocker leaves, and then deals its combat damage to nothing
    /// at all. The engine has carried that as a flag since a blinked blocker
    /// let an attacker through; without it here a client derives "blocked"
    /// from an empty blocker list, draws no line, and counts the damage
    /// against the player it never reaches.
    pub blocked: bool,
}

/// One declared blocker and the attacker it blocks.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct BlockerView {
    /// The blocking creature.
    pub blocker: ObjectId,
    /// The attacker it blocks.
    pub attacker: ObjectId,
}

/// Declared combat, used to draw attack and block arrows.
#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct CombatView {
    /// Declared attackers.
    pub attackers: Vec<AttackerView>,
    /// Declared blockers.
    pub blockers: Vec<BlockerView>,
    /// The attacking bands (CR 702.22c), each its members in declaration
    /// order. Public: the attacking player announces them (CR 508.1e), and
    /// blocking one member blocks the band (CR 702.22h).
    #[serde(default)]
    pub bands: Vec<Vec<ObjectId>>,
}

impl CombatView {
    /// Whether any creature is attacking.
    #[must_use]
    pub fn is_active(&self) -> bool {
        !self.attackers.is_empty()
    }

    /// Everything blocking a given attacker.
    pub fn blockers_of(&self, attacker: ObjectId) -> impl Iterator<Item = ObjectId> + '_ {
        self.blockers
            .iter()
            .filter(move |b| b.attacker == attacker)
            .map(|b| b.blocker)
    }

    /// Whether an attacker is unblocked, which a client marks because it
    /// decides whether damage reaches the defending player.
    ///
    /// Asks the flag and not the blocker list: an attacker whose blockers
    /// have all left is blocked and dealing damage to nobody (CR 509.1h),
    /// and an arithmetic that read the list would hand the whole squad's
    /// damage to the player it is not reaching.
    #[must_use]
    pub fn is_unblocked(&self, attacker: ObjectId) -> bool {
        !self
            .attackers
            .iter()
            .any(|a| a.creature == attacker && a.blocked)
    }
}
