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
