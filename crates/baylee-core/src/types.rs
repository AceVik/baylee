//! Card types, supertypes, and subtype identifiers.
//!
//! Types are compact bitsets for fast rules evaluation; subtype constants
//! are generated from Scryfall catalogs into `crate::generated::subtypes`.

use crate::ids::SubtypeId;
use serde::{Deserialize, Serialize};

/// Const-friendly string equality (`str` cannot be matched in const fn).
pub(crate) const fn str_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// Bitset of card types (CR 205.2).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct TypeSet(u16);

impl TypeSet {
    /// No types.
    pub const EMPTY: Self = Self(0);
    /// Artifact.
    pub const ARTIFACT: Self = Self(1 << 0);
    /// Creature.
    pub const CREATURE: Self = Self(1 << 1);
    /// Enchantment.
    pub const ENCHANTMENT: Self = Self(1 << 2);
    /// Instant.
    pub const INSTANT: Self = Self(1 << 3);
    /// Kindred (formerly "Tribal").
    pub const KINDRED: Self = Self(1 << 4);
    /// Land.
    pub const LAND: Self = Self(1 << 5);
    /// Planeswalker.
    pub const PLANESWALKER: Self = Self(1 << 6);
    /// Sorcery.
    pub const SORCERY: Self = Self(1 << 7);
    /// Battle.
    pub const BATTLE: Self = Self(1 << 8);
    /// Dungeon.
    pub const DUNGEON: Self = Self(1 << 9);
    /// Plane.
    pub const PLANE: Self = Self(1 << 10);
    /// Scheme.
    pub const SCHEME: Self = Self(1 << 11);
    /// Vanguard.
    pub const VANGUARD: Self = Self(1 << 12);
    /// Phenomenon.
    pub const PHENOMENON: Self = Self(1 << 13);
    /// Conspiracy.
    pub const CONSPIRACY: Self = Self(1 << 14);
    /// Attraction (silver-bordered).
    pub const ATTRACTION: Self = Self(1 << 15);

    /// Parses a type word from a type line ("Artifact", "Creature", …).
    #[must_use]
    pub const fn from_word(word: &str) -> Option<Self> {
        if str_eq(word, "Artifact") {
            Some(Self::ARTIFACT)
        } else if str_eq(word, "Creature") {
            Some(Self::CREATURE)
        } else if str_eq(word, "Enchantment") {
            Some(Self::ENCHANTMENT)
        } else if str_eq(word, "Instant") {
            Some(Self::INSTANT)
        } else if str_eq(word, "Kindred") || str_eq(word, "Tribal") {
            Some(Self::KINDRED)
        } else if str_eq(word, "Land") {
            Some(Self::LAND)
        } else if str_eq(word, "Planeswalker") {
            Some(Self::PLANESWALKER)
        } else if str_eq(word, "Sorcery") {
            Some(Self::SORCERY)
        } else if str_eq(word, "Battle") {
            Some(Self::BATTLE)
        } else if str_eq(word, "Dungeon") {
            Some(Self::DUNGEON)
        } else if str_eq(word, "Plane") {
            Some(Self::PLANE)
        } else if str_eq(word, "Scheme") {
            Some(Self::SCHEME)
        } else if str_eq(word, "Vanguard") {
            Some(Self::VANGUARD)
        } else if str_eq(word, "Phenomenon") {
            Some(Self::PHENOMENON)
        } else if str_eq(word, "Conspiracy") {
            Some(Self::CONSPIRACY)
        } else if str_eq(word, "Attraction") {
            Some(Self::ATTRACTION)
        } else {
            None
        }
    }

    /// Whether the set contains the given type.
    #[inline]
    #[must_use]
    pub const fn contains(self, t: Self) -> bool {
        self.0 & t.0 != 0
    }

    /// Union.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Intersection.
    #[must_use]
    pub const fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    /// Whether both sets share at least one type.
    #[inline]
    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    /// Difference.
    #[must_use]
    pub const fn difference(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }

    /// Whether no types are set.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Raw bits.
    #[must_use]
    pub const fn bits(self) -> u16 {
        self.0
    }

    /// Whether this describes a permanent type (CR 110.4).
    #[must_use]
    pub const fn is_permanent(self) -> bool {
        self.intersects(
            Self::ARTIFACT
                .union(Self::CREATURE)
                .union(Self::ENCHANTMENT)
                .union(Self::LAND)
                .union(Self::PLANESWALKER)
                .union(Self::BATTLE),
        )
    }

    /// Whether this describes a spell type (instant/sorcery/kindred spells).
    #[must_use]
    pub const fn is_instant_or_sorcery(self) -> bool {
        self.intersects(Self::INSTANT.union(Self::SORCERY))
    }
}

impl core::fmt::Debug for TypeSet {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "TypeSet({:#06x})", self.0)
    }
}

/// Bitset of supertypes (CR 205.4).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct SupertypeSet(u8);

impl SupertypeSet {
    /// No supertypes.
    pub const EMPTY: Self = Self(0);
    /// Basic.
    pub const BASIC: Self = Self(1 << 0);
    /// Legendary.
    pub const LEGENDARY: Self = Self(1 << 1);
    /// Snow.
    pub const SNOW: Self = Self(1 << 2);
    /// World.
    pub const WORLD: Self = Self(1 << 3);
    /// Ongoing.
    pub const ONGOING: Self = Self(1 << 4);
    /// Host.
    pub const HOST: Self = Self(1 << 5);

    /// Parses a supertype word ("Basic", "Legendary", …).
    #[must_use]
    pub const fn from_word(word: &str) -> Option<Self> {
        if str_eq(word, "Basic") {
            Some(Self::BASIC)
        } else if str_eq(word, "Legendary") {
            Some(Self::LEGENDARY)
        } else if str_eq(word, "Snow") {
            Some(Self::SNOW)
        } else if str_eq(word, "World") {
            Some(Self::WORLD)
        } else if str_eq(word, "Ongoing") {
            Some(Self::ONGOING)
        } else if str_eq(word, "Host") {
            Some(Self::HOST)
        } else {
            None
        }
    }

    /// Whether the set contains the given supertype.
    #[inline]
    #[must_use]
    pub const fn contains(self, t: Self) -> bool {
        self.0 & t.0 != 0
    }

    /// Union.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Whether no supertypes are set.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Raw bits.
    #[must_use]
    pub const fn bits(self) -> u8 {
        self.0
    }
}

impl core::fmt::Debug for SupertypeSet {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "SupertypeSet({:#04x})", self.0)
    }
}

/// The kind a subtype belongs to (determines which type line it may appear on).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum SubtypeKind {
    /// Creature subtypes ("Elf", "Ally", …).
    Creature,
    /// Artifact subtypes ("Equipment", "Clue", …).
    Artifact,
    /// Enchantment subtypes ("Aura", "Saga", …).
    Enchantment,
    /// Land subtypes ("Forest", "Urza's", …).
    Land,
    /// Planeswalker subtypes ("Jace", "Teferi", …).
    Planeswalker,
    /// Spell subtypes ("Arcane", "Lesson", …).
    Spell,
}

/// 512-bit subtype bitmap.
///
/// Fixed-size so "has all creature types" (changeling, Maskwood Nexus) is a
/// single `set_all` and membership tests are one bitmask operation.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct SubtypeSet([u64; 8]);

impl SubtypeSet {
    /// No subtypes.
    pub const EMPTY: Self = Self([0; 8]);
    /// All 512 possible subtypes set.
    pub const ALL: Self = Self([u64::MAX; 8]);

    /// Whether the subtype is present.
    #[inline]
    #[must_use]
    pub const fn contains(self, id: SubtypeId) -> bool {
        let word = (id.get() / 64) as usize;
        let bit = id.get() % 64;
        (self.0[word] >> bit) & 1 == 1
    }

    /// Adds a subtype.
    pub const fn insert(&mut self, id: SubtypeId) {
        let word = (id.get() / 64) as usize;
        let bit = id.get() % 64;
        self.0[word] |= 1 << bit;
    }

    /// Removes a subtype.
    pub const fn remove(&mut self, id: SubtypeId) {
        let word = (id.get() / 64) as usize;
        let bit = id.get() % 64;
        self.0[word] &= !(1 << bit);
    }

    /// Adds all subtypes of `other`.
    pub const fn union_with(&mut self, other: Self) {
        let mut i = 0;
        while i < 8 {
            self.0[i] |= other.0[i];
            i += 1;
        }
    }

    /// Whether no subtype is set.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        let mut i = 0;
        while i < 8 {
            if self.0[i] != 0 {
                return false;
            }
            i += 1;
        }
        true
    }

    /// A set built from a slice of subtypes (const-friendly for codegen).
    #[must_use]
    pub const fn from_slice(ids: &[SubtypeId]) -> Self {
        let mut set = Self::EMPTY;
        let mut i = 0;
        while i < ids.len() {
            set.insert(ids[i]);
            i += 1;
        }
        set
    }

    /// Raw 64-bit words (snapshot hashing).
    #[must_use]
    pub const fn words(&self) -> &[u64; 8] {
        &self.0
    }
}

impl core::fmt::Debug for SubtypeSet {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "SubtypeSet(..)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_words() {
        assert_eq!(TypeSet::from_word("Creature"), Some(TypeSet::CREATURE));
        assert!(TypeSet::CREATURE.is_permanent());
        assert!(!TypeSet::INSTANT.is_permanent());
        assert!(SupertypeSet::from_word("Legendary").is_some());
    }

    #[test]
    fn subtype_bitmap() {
        let mut set = SubtypeSet::EMPTY;
        let id = SubtypeId::new(300);
        assert!(!set.contains(id));
        set.insert(id);
        assert!(set.contains(id));
        assert!(SubtypeSet::ALL.contains(SubtypeId::new(511)));
    }
}
