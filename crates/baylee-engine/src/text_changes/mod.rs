//! Typed word substitution for layer 3 (CR 612.1–4, 613.1c).
//!
//! These maps change semantic words in printed or copied rules. Mana symbols,
//! names, color indicators and abilities granted in layer 6 are never inputs.

use baylee_cards_dsl::{Filter, KeywordSet, Modifier, TextWordKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::land;
use baylee_core::ids::{DamageSourceRef, ObjectId, SubtypeId};
use baylee_core::types::SubtypeSet;

/// The five basic land types in the same WUBRG order as colors.
pub const BASIC_LAND_TYPES: [SubtypeId; 5] = [
    land::PLAINS,
    land::ISLAND,
    land::SWAMP,
    land::MOUNTAIN,
    land::FOREST,
];

const LANDWALK: [KeywordSet; 5] = [
    KeywordSet::PLAINSWALK,
    KeywordSet::ISLANDWALK,
    KeywordSet::SWAMPWALK,
    KeywordSet::MOUNTAINWALK,
    KeywordSet::FORESTWALK,
];

/// A choice of two distinct words within one semantic family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextReplacement {
    /// Color words or basic land type words.
    pub kind: TextWordKind,
    /// Original word's WUBRG index.
    pub from: u8,
    /// Replacement word's WUBRG index.
    pub to: u8,
}

impl TextReplacement {
    /// Validate an answer before it changes any state.
    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.from < 5 && self.to < 5 && self.from != self.to
    }

    /// Decode one of the twenty distinct ordered pairs, in WUBRG order.
    #[must_use]
    pub fn from_choice(kind: TextWordKind, choice: u32) -> Option<Self> {
        if choice >= 20 {
            return None;
        }
        let from = u8::try_from(choice / 4).ok()?;
        let offset = u8::try_from(choice % 4).ok()?;
        let to = if offset >= from { offset + 1 } else { offset };
        Some(Self { kind, from, to })
    }
}

/// A composition of all text changes affecting one rules incarnation.
/// Ten bytes, deterministic and cheap to capture on a stack ability.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextChangeMap {
    colors: [u8; 5],
    lands: [u8; 5],
}

impl Default for TextChangeMap {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl TextChangeMap {
    /// Unchanged printed or copied text.
    pub const IDENTITY: Self = Self {
        colors: [0, 1, 2, 3, 4],
        lands: [0, 1, 2, 3, 4],
    };

    /// Compose a later replacement after the changes already present.
    /// The same word, an invalid word, or a mismatched family cannot mutate it.
    pub fn replace(&mut self, replacement: TextReplacement) -> bool {
        if !replacement.is_valid() {
            return false;
        }
        let words = match replacement.kind {
            TextWordKind::Color => &mut self.colors,
            TextWordKind::BasicLandType => &mut self.lands,
        };
        for word in words {
            if *word == replacement.from {
                *word = replacement.to;
            }
        }
        true
    }

    /// Apply a second word map after this one. Used when already-defined
    /// token text receives a later text-changing effect.
    #[must_use]
    pub fn then(self, later: Self) -> Self {
        Self {
            colors: self.colors.map(|word| later.colors[usize::from(word)]),
            lands: self.lands.map(|word| later.lands[usize::from(word)]),
        }
    }

    /// A color used as a word, never a mana symbol or color indicator.
    #[must_use]
    pub fn color_word(self, color: Color) -> Color {
        Color::ALL[usize::from(self.colors[color as usize])]
    }

    /// Map every color word in a filter or an explicit color-setting effect.
    #[must_use]
    pub fn color_words(self, colors: ColorSet) -> ColorSet {
        Color::ALL
            .into_iter()
            .fold(ColorSet::EMPTY, |mapped, color| {
                if colors.contains(color) {
                    mapped.union(ColorSet::of(self.color_word(color)))
                } else {
                    mapped
                }
            })
    }

    /// Map a basic land type word; other subtypes retain their identity.
    #[must_use]
    pub fn land_type(self, subtype: SubtypeId) -> SubtypeId {
        BASIC_LAND_TYPES
            .iter()
            .position(|original| *original == subtype)
            .map_or(subtype, |index| {
                BASIC_LAND_TYPES[usize::from(self.lands[index])]
            })
    }

    /// Map the type line or token-defined subtype words simultaneously.
    #[must_use]
    pub fn land_types(self, subtypes: SubtypeSet) -> SubtypeSet {
        let mut mapped = subtypes;
        for original in BASIC_LAND_TYPES {
            mapped.remove(original);
        }
        for original in BASIC_LAND_TYPES {
            if subtypes.contains(original) {
                mapped.insert(self.land_type(original));
            }
        }
        mapped
    }

    /// Basic landwalk includes a land type word (CR 702.14c).
    /// Other keyword reminder text is not rules text to be substituted.
    #[must_use]
    pub fn keywords(self, keywords: KeywordSet) -> KeywordSet {
        let all = LANDWALK
            .into_iter()
            .fold(KeywordSet::EMPTY, KeywordSet::union);
        let mut mapped = keywords.difference(all);
        for (index, keyword) in LANDWALK.into_iter().enumerate() {
            if keywords.contains(keyword) {
                mapped = mapped.union(LANDWALK[usize::from(self.lands[index])]);
            }
        }
        mapped
    }

    /// Substitute the typed leaves at each recursive filter visit.
    /// Names and the semantics hidden inside a keyword are deliberately intact.
    #[must_use]
    pub fn filter_leaf(self, filter: Filter) -> Filter {
        match filter {
            Filter::HasColor(colors) => Filter::HasColor(self.color_words(colors)),
            Filter::HasSubtype(subtype) => Filter::HasSubtype(self.land_type(subtype)),
            Filter::HasKeyword(keywords) => Filter::HasKeyword(self.keywords(keywords)),
            other => other,
        }
    }

    /// Immediate literal operands. Continuous-effect operands and borrowed
    /// filters remain raw and carry this context to their eventual reader.
    /// Mana colors encode symbols or dynamically chosen values, never words.
    #[must_use]
    pub fn effect_immediate(self, effect: baylee_cards_dsl::Effect) -> baylee_cards_dsl::Effect {
        use baylee_cards_dsl::Effect;
        match effect {
            Effect::GrantSubtype { subtype } => Effect::GrantSubtype {
                subtype: self.land_type(subtype),
            },
            Effect::Amass {
                token,
                subtype,
                amount,
            } => Effect::Amass {
                token,
                subtype: self.land_type(subtype),
                amount,
            },
            other => other,
        }
    }

    /// Direct typed operands; nested filters use this same map when evaluated.
    #[must_use]
    pub fn modifier(self, modifier: Modifier) -> Modifier {
        match modifier {
            Modifier::AddColor(colors) => Modifier::AddColor(self.color_words(colors)),
            Modifier::SetColor(colors) => Modifier::SetColor(self.color_words(colors)),
            Modifier::AddSubtype(subtype) => Modifier::AddSubtype(self.land_type(subtype)),
            Modifier::SetLandType(subtype) => Modifier::SetLandType(self.land_type(subtype)),
            Modifier::ReplaceCreatureTypes(subtype) => {
                Modifier::ReplaceCreatureTypes(self.land_type(subtype))
            }
            Modifier::BecomeType { types, subtype } => Modifier::BecomeType {
                types,
                subtype: self.land_type(subtype),
            },
            Modifier::AddKeyword(keywords) => Modifier::AddKeyword(self.keywords(keywords)),
            Modifier::RemoveKeyword(keywords) => Modifier::RemoveKeyword(self.keywords(keywords)),
            Modifier::AddKeywordIfCountersAtLeast {
                kind,
                at_least,
                keywords,
            } => Modifier::AddKeywordIfCountersAtLeast {
                kind,
                at_least,
                keywords: self.keywords(keywords),
            },
            other => other,
        }
    }
}

/// Text affecting a live object, or frozen when an ability went on the stack.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextChangeEntry {
    /// Exact rules identity; blinking creates a different one.
    pub object: DamageSourceRef,
    /// Ordered composition of all relevant substitutions.
    pub map: TextChangeMap,
}

/// Sparse text changes, kept off every ordinary object's hot representation.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct TextChanges {
    entries: Vec<TextChangeEntry>,
}

impl TextChanges {
    /// Deterministic insertion order, suitable for snapshots and fingerprints.
    #[must_use]
    pub fn entries(&self) -> &[TextChangeEntry] {
        &self.entries
    }

    /// Unchanged text when this incarnation has no entry.
    #[must_use]
    pub fn get(&self, object: DamageSourceRef) -> TextChangeMap {
        self.entries
            .iter()
            .find(|entry| entry.object == object)
            .map_or(TextChangeMap::IDENTITY, |entry| entry.map)
    }

    /// Install a snapshot; identity maps need no storage.
    pub fn set(&mut self, object: DamageSourceRef, map: TextChangeMap) {
        if map == TextChangeMap::IDENTITY {
            self.remove(object);
        } else if let Some(entry) = self.entries.iter_mut().find(|entry| entry.object == object) {
            entry.map = map;
        } else {
            self.entries.push(TextChangeEntry { object, map });
        }
    }

    /// Add a resolving text change, refusing malformed answers atomically.
    pub fn replace(&mut self, object: DamageSourceRef, replacement: TextReplacement) -> bool {
        let mut map = self.get(object);
        if !map.replace(replacement) {
            return false;
        }
        self.set(object, map);
        true
    }

    /// A permanent spell keeps its text changes as it resolves (CR 400.7b).
    pub fn carry(&mut self, previous: DamageSourceRef, entered: DamageSourceRef) {
        let map = self.get(previous);
        self.remove(previous);
        self.set(entered, map);
    }

    /// Drop one departed incarnation once its last-known text was captured.
    pub fn remove(&mut self, object: DamageSourceRef) {
        self.entries.retain(|entry| entry.object != object);
    }

    /// Drop historical wording only after all live readers have been collected.
    pub(crate) fn retain(&mut self, mut needed: impl FnMut(DamageSourceRef) -> bool) {
        self.entries.retain(|entry| needed(entry.object));
    }
}

/// Rules source identity and text are distinct: a stack ability still refers
/// to its old source, while its words stopped changing when it was created.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RuleContext {
    /// What "this object" identifies.
    pub source: ObjectId,
    /// The semantic text of the particular rules being evaluated.
    pub text: TextChangeMap,
}

/// Where a derived effect obtains its semantic text, independently of its
/// rules source. A granted ability refers to its recipient as "this" while
/// its wording comes from the grantor (CR 612.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextOrigin {
    /// A static ability still follows the text of the exact granting object.
    Live(DamageSourceRef),
    /// A printed/copied ability, including wording defined at token creation.
    Ability {
        /// Exact current source incarnation.
        source: DamageSourceRef,
        /// Runtime ability index, preserving independent duplicate instances.
        index: u32,
        /// Copiable wording before later text-changing effects.
        base: TextChangeMap,
    },
    /// A resolving spell or captured ability no longer follows its source.
    Frozen(TextChangeMap),
}

impl TextOrigin {
    /// Read the current wording for this origin.
    #[must_use]
    pub fn resolve(self, changes: &TextChanges) -> TextChangeMap {
        match self {
            Self::Live(object) => changes.get(object),
            Self::Ability { source, base, .. } => base.then(changes.get(source)),
            Self::Frozen(map) => map,
        }
    }
}

#[cfg(test)]
mod tests;
