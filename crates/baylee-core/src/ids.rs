//! Typed integer handles used across the whole platform.
//!
//! Rules code never touches strings; every entity is addressed by a compact,
//! copyable id. All ids are `Copy + Ord + Hash` and serialize transparently.

use serde::{Deserialize, Serialize};

macro_rules! id_type {
    ($($(#[$meta:meta])* $name:ident($inner:ty);)*) => {
        $(
            $(#[$meta])*
            #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
            #[serde(transparent)]
            #[repr(transparent)]
            pub struct $name(pub $inner);

            impl $name {
                /// Creates the id from its raw value.
                #[inline]
                #[must_use]
                pub const fn new(value: $inner) -> Self {
                    Self(value)
                }

                /// Returns the raw value.
                #[inline]
                #[must_use]
                pub const fn get(self) -> $inner {
                    self.0
                }
            }

            impl core::fmt::Debug for $name {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    write!(f, concat!(stringify!($name), "({})"), self.0)
                }
            }

            impl core::fmt::Display for $name {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    write!(f, "{}", self.0)
                }
            }
        )*
    };
}

id_type! {
    /// Rules identity of a card definition (oracle-keyed, assigned by codegen).
    CardIndex(u32);
    /// Opaque index into a game's print table (presentation-only payload).
    PrintRef(u16);
    /// Player/seat handle within a game.
    PlayerId(u8);
    /// Handle of a registered continuous effect.
    EffectId(u32);
    /// Handle of a triggered ability instance.
    TriggerId(u32);
    /// Handle of an ability instance on the stack.
    AbilityInstanceId(u32);
    /// Subtype identifier; constants live in `crate::generated::subtypes`.
    SubtypeId(u16);
    /// Interned name handle (rules identity for "cards named X", not display).
    NameRef(u32);
}

/// One ability of one card, addressed the same way in every game.
///
/// Unlike a game object's ability instance, this carries no per-game
/// handle: it is `(which card, which ability of it)` and nothing else.
/// That is what lets a client label a stack entry ("Ondu Cleric's rally
/// trigger") and what lets a player's standing answer for an ability —
/// "always yes for this one" — be stored once by an account and replayed
/// into every future game.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct AbilityRef {
    /// The card the ability is printed on.
    pub card: CardIndex,
    /// Index into that card's ability list.
    pub index: u32,
}

impl AbilityRef {
    /// Builds a handle.
    #[inline]
    #[must_use]
    pub const fn new(card: CardIndex, index: u32) -> Self {
        Self { card, index }
    }

    /// The card's spell ability — what an instant or sorcery does when it
    /// resolves, which is not an entry in its ability list.
    pub const SPELL: u32 = u32::MAX;
    /// The card's as-it-enters question (a shockland's "pay 2 life?").
    pub const ENTERS: u32 = u32::MAX - 1;
    /// The card's optional additional cost at cast time (kicker).
    pub const ADDITIONAL_COST: u32 = u32::MAX - 2;
    /// The card's miracle offer (CR 702.94).
    pub const MIRACLE: u32 = u32::MAX - 3;
    /// A recurring upkeep payment the card imposes (echo, pacts).
    pub const UPKEEP_COST: u32 = u32::MAX - 4;

    /// The lowest reserved index. Real ability indices are positions in a
    /// card's ability list and never come close; reserving the top of the
    /// range lets card-level questions that are not abilities — a
    /// shockland's entry choice, a kicker — be addressed by the same
    /// handle, so "always yes for this card's question" works for them
    /// too.
    pub const FIRST_RESERVED: u32 = u32::MAX - 4;

    /// Whether this handle names a real entry in the card's ability list.
    #[must_use]
    pub const fn is_listed_ability(self) -> bool {
        self.index < Self::FIRST_RESERVED
    }
}

impl core::fmt::Display for AbilityRef {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "card {}#{}", self.card.get(), self.index)
    }
}

/// Generational arena handle for a game object: `slot:24 | generation:8`.
///
/// The generation distinguishes an object from earlier objects that occupied
/// the same arena slot (e.g. after zone changes into hidden zones).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct ObjectId(u32);

impl ObjectId {
    /// Mask for the slot portion.
    pub const SLOT_MASK: u32 = 0x00FF_FFFF;
    /// Highest addressable arena slot (~16.7M objects per game).
    pub const MAX_SLOT: u32 = Self::SLOT_MASK;

    /// Packs a slot and a generation into one handle.
    ///
    /// # Panics
    /// When `slot` exceeds [`ObjectId::MAX_SLOT`].
    #[inline]
    #[must_use]
    pub const fn new(slot: u32, generation: u8) -> Self {
        assert!(slot <= Self::SLOT_MASK, "ObjectId slot out of range");
        Self(((generation as u32) << 24) | (slot & Self::SLOT_MASK))
    }

    /// The arena slot.
    #[inline]
    #[must_use]
    pub const fn slot(self) -> u32 {
        self.0 & Self::SLOT_MASK
    }

    /// The generation.
    #[inline]
    #[must_use]
    pub const fn generation(self) -> u8 {
        (self.0 >> 24) as u8
    }

    /// The same slot with the generation incremented (wrapping).
    #[inline]
    #[must_use]
    pub const fn bumped(self) -> Self {
        Self::new(self.slot(), self.generation().wrapping_add(1))
    }
}

impl core::fmt::Debug for ObjectId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "ObjectId({}#{})", self.slot(), self.generation())
    }
}

impl core::fmt::Display for ObjectId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}#{}", self.slot(), self.generation())
    }
}

/// What an attack is aimed at (CR 506.2).
///
/// A creature does not attack a *player*; it attacks a **defender**, and
/// the defending player's planeswalkers are defenders too (CR 508.1a).
/// Modelling that as a plain [`PlayerId`] made planeswalkers unattackable
/// by construction, which is why this handle exists. Battles (CR 310) join
/// the enum as a third case when they arrive; every match on it is written
/// so that adding one is a compile error rather than a silent fallthrough.
///
/// It lives in `baylee-core` because both halves of the wire — the engine's
/// `PlayerAction` and the client-facing view — have to name the same thing.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub enum Defender {
    /// The defending player themself.
    Player(PlayerId),
    /// A planeswalker the defending player controls.
    Planeswalker(ObjectId),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_id_roundtrip() {
        let id = ObjectId::new(42, 7);
        assert_eq!(id.slot(), 42);
        assert_eq!(id.generation(), 7);
        assert_eq!(id.bumped().generation(), 8);
    }
}
