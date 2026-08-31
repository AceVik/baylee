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

    /// The type words in the set, in the order a card prints them.
    ///
    /// Printed order is not the bit order and not alphabetical: a card reads
    /// "Artifact Creature", never "Creature Artifact", and "Land Creature"
    /// (Dryad Arbor) rather than the reverse. The order below is that printed
    /// order, so a constructed type line matches the physical card.
    pub fn words(self) -> impl Iterator<Item = &'static str> {
        const PRINTED_ORDER: [(TypeSet, &str); 16] = [
            (TypeSet::KINDRED, "Kindred"),
            (TypeSet::ARTIFACT, "Artifact"),
            (TypeSet::ENCHANTMENT, "Enchantment"),
            (TypeSet::LAND, "Land"),
            (TypeSet::CREATURE, "Creature"),
            (TypeSet::PLANESWALKER, "Planeswalker"),
            (TypeSet::BATTLE, "Battle"),
            (TypeSet::INSTANT, "Instant"),
            (TypeSet::SORCERY, "Sorcery"),
            (TypeSet::DUNGEON, "Dungeon"),
            (TypeSet::PLANE, "Plane"),
            (TypeSet::PHENOMENON, "Phenomenon"),
            (TypeSet::SCHEME, "Scheme"),
            (TypeSet::VANGUARD, "Vanguard"),
            (TypeSet::CONSPIRACY, "Conspiracy"),
            (TypeSet::ATTRACTION, "Attraction"),
        ];
        PRINTED_ORDER
            .into_iter()
            .filter(move |(t, _)| self.contains(*t))
            .map(|(_, w)| w)
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

    /// Difference.
    #[must_use]
    pub const fn difference(self, other: Self) -> Self {
        Self(self.0 & !other.0)
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

    /// The supertype words in the set, in printed order (CR 205.4a — they
    /// come before every card type: "Legendary Snow Artifact").
    pub fn words(self) -> impl Iterator<Item = &'static str> {
        const PRINTED_ORDER: [(SupertypeSet, &str); 6] = [
            (SupertypeSet::BASIC, "Basic"),
            (SupertypeSet::LEGENDARY, "Legendary"),
            (SupertypeSet::ONGOING, "Ongoing"),
            (SupertypeSet::SNOW, "Snow"),
            (SupertypeSet::WORLD, "World"),
            (SupertypeSet::HOST, "Host"),
        ];
        PRINTED_ORDER
            .into_iter()
            .filter(move |(t, _)| self.contains(*t))
            .map(|(_, w)| w)
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

    /// Union of two sets.
    #[inline]
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        let mut out = [0u64; 8];
        let mut i = 0;
        while i < 8 {
            out[i] = self.0[i] | other.0[i];
            i += 1;
        }
        Self(out)
    }

    /// Whether the two sets share at least one subtype.
    ///
    /// Word-wise: eight `AND`s instead of one probe per subtype id. The
    /// naive loop over [`COUNT`](crate::generated::subtypes::COUNT) ids was
    /// 500+ iterations for a question eight instructions answer.
    #[inline]
    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        let mut i = 0;
        while i < 8 {
            if self.0[i] & other.0[i] != 0 {
                return true;
            }
            i += 1;
        }
        false
    }

    /// The set of all subtype ids in `start..end`.
    ///
    /// Subtype ids are range-partitioned per kind by codegen, so "every
    /// creature type" (changeling, CR 702.73) is one contiguous range and
    /// therefore a compile-time constant rather than a runtime scan.
    #[must_use]
    pub const fn range(start: u16, end: u16) -> Self {
        let mut set = Self::EMPTY;
        let mut i = start;
        while i < end {
            set.insert(SubtypeId::new(i));
            i += 1;
        }
        set
    }

    /// Number of subtypes in the set.
    #[must_use]
    pub const fn len(self) -> u32 {
        let mut n = 0;
        let mut i = 0;
        while i < 8 {
            n += self.0[i].count_ones();
            i += 1;
        }
        n
    }

    /// Raw 64-bit words (snapshot hashing).
    #[must_use]
    pub const fn words(&self) -> &[u64; 8] {
        &self.0
    }

    /// Whether every subtype in `other` is also in this set.
    #[must_use]
    pub const fn contains_all(self, other: Self) -> bool {
        let mut i = 0;
        while i < 8 {
            if self.0[i] & other.0[i] != other.0[i] {
                return false;
            }
            i += 1;
        }
        true
    }

    /// The subtypes in the set, in ascending id order.
    ///
    /// Ascending id order is also kind order (creature types, then artifact,
    /// then …), which is what a type line wants: ids are assigned per kind by
    /// codegen. Iteration walks set bits rather than all 512 slots, so a card
    /// with two subtypes costs two steps, not five hundred.
    pub fn iter(self) -> impl Iterator<Item = SubtypeId> {
        (0..8).flat_map(move |word| {
            let mut bits = self.0[word];
            core::iter::from_fn(move || {
                if bits == 0 {
                    return None;
                }
                let bit = bits.trailing_zeros();
                bits &= bits - 1;
                Some(SubtypeId::new((word as u16) * 64 + bit as u16))
            })
        })
    }

    /// The five basic land types (CR 305.6).
    ///
    /// Not a codegen range — the basics are five specific ids inside the
    /// land block, not the whole block (Desert and Gate are land types
    /// too, and Dryad Arbor does not have them).
    pub const BASIC_LANDS: Self = Self::from_slice(&[
        crate::generated::subtypes::land::PLAINS,
        crate::generated::subtypes::land::ISLAND,
        crate::generated::subtypes::land::SWAMP,
        crate::generated::subtypes::land::MOUNTAIN,
        crate::generated::subtypes::land::FOREST,
    ]);

    /// Every creature type — what changeling grants (CR 702.73).
    pub const ALL_CREATURE: Self = crate::generated::subtypes::ALL_CREATURE_TYPES;
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

    /// `iter` is what a client turns a projected object into a type line
    /// with, so it has to walk set bits in ascending id order and nothing
    /// else — an out-of-order or over-long iteration would misprint every
    /// type line rather than fail loudly.
    #[test]
    fn subtypes_iterate_in_ascending_id_order() {
        use crate::generated::subtypes;
        let set = SubtypeSet::from_slice(&[
            subtypes::land::FOREST,
            subtypes::creature::WIZARD,
            subtypes::creature::ALLY,
        ]);
        let ids: Vec<_> = set.iter().collect();
        assert_eq!(ids.len(), 3);
        assert!(ids.windows(2).all(|w| w[0].get() < w[1].get()));
        assert_eq!(ids[0], subtypes::creature::ALLY);
        assert_eq!(ids[2], subtypes::land::FOREST);
        assert_eq!(SubtypeSet::EMPTY.iter().count(), 0);
        assert_eq!(
            SubtypeSet::ALL_CREATURE.iter().count(),
            SubtypeSet::ALL_CREATURE.len() as usize
        );
    }

    /// Changeling grants every creature type; a client has to be able to ask
    /// "is this the all-types case" cheaply so it can collapse the type line
    /// instead of printing three hundred words.
    #[test]
    fn contains_all_recognises_the_changeling_case() {
        use crate::generated::subtypes;
        assert!(SubtypeSet::ALL_CREATURE.contains_all(SubtypeSet::ALL_CREATURE));
        assert!(
            SubtypeSet::ALL_CREATURE
                .contains_all(SubtypeSet::from_slice(&[subtypes::creature::WIZARD]))
        );
        assert!(
            !SubtypeSet::from_slice(&[subtypes::creature::WIZARD])
                .contains_all(SubtypeSet::ALL_CREATURE)
        );
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
