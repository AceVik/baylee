//! Declarative object filters — the targeting/selection algebra.
//!
//! Filters are pure data, composable, and const-constructible, so cards can
//! declare them in `static`s and the engine can evaluate them without any
//! per-card code. `you` in evaluations is the controller of the ability or
//! spell; `this` is its source object.

use crate::KeywordSet;
use baylee_core::color::ColorSet;
use baylee_core::ids::SubtypeId;
use baylee_core::types::{SupertypeSet, TypeSet};

/// A predicate over game objects.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Filter {
    /// Every object matches.
    Any,
    /// The source object itself.
    This,
    /// Anything except the source object ("another").
    Another,
    /// All must match.
    And(&'static [Filter]),
    /// At least one must match.
    Or(&'static [Filter]),
    /// Must not match.
    Not(&'static Filter),
    /// Has all of these types.
    HasType(TypeSet),
    /// Has none of these types.
    LacksType(TypeSet),
    /// Has all of these supertypes.
    HasSupertype(SupertypeSet),
    /// Has this subtype.
    HasSubtype(SubtypeId),
    /// Has at least one of these colors.
    HasColor(ColorSet),
    /// Is colorless.
    IsColorless,
    /// Exactly one color (Vanishing Verse).
    Monocolored,
    /// Is a token (Sheoldred's Edict: "creature token").
    IsToken,
    /// Controlled by `you`.
    ControlledByYou,
    /// Controlled by an opponent of `you`.
    ControlledByOpponent,
    /// Owned by `you`.
    OwnedByYou,
    /// Currently tapped.
    Tapped,
    /// Currently untapped.
    Untapped,
    /// Currently attacking (in combat).
    Attacking,
    /// Has the subtype the SOURCE object chose as it entered ("the chosen
    /// type" — Roaming Throne, Reflections of Littjara, Cavern of Souls).
    MatchesChosenTypeOfSource,
    /// Shares at least one creature subtype with your commander (Path of
    /// Ancestry's scry rider).
    SharesSubtypeWithCommander,
    /// The object the SOURCE is attached to (equipment/auras): matches the
    /// creature the source is attached to.
    AttachedToBySource,
    /// Has this keyword.
    HasKeyword(KeywordSet),
    /// Converted mana cost at most N.
    CmcAtMost(u32),
    /// Converted mana cost at least N.
    CmcAtLeast(u32),
    /// Toughness at most N (Recruiter of the Guard).
    ToughnessAtMost(i16),
    /// Is in the given zone (cross-zone effects like Maskwood Nexus).
    InZone(ZoneRef),
}

/// Zone references for filters (engine zones, DSL view).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ZoneRef {
    /// The battlefield.
    Battlefield,
    /// The stack.
    Stack,
    /// A library.
    Library,
    /// A hand.
    Hand,
    /// A graveyard.
    Graveyard,
    /// Exile.
    Exile,
    /// The command zone.
    Command,
    /// Any zone except the battlefield ("cards you own that aren't on the
    /// battlefield").
    NotBattlefield,
}

impl Filter {
    // Compose filters inline (`Filter::And(&[A, B])`) — in `static` context
    // the slice promotes to `'static` automatically, so a card needs a
    // `static` of its own only for a filter it refers to more than once.
    //
    // The constants below are the ones the pool kept reinventing: "a
    // creature" was written out as `HasType(TypeSet::CREATURE)` in a
    // differently-named `static` in twenty-six card files, which is
    // twenty-six chances to write `LacksType` by accident and no way to
    // grep for the ones that did.

    /// A creature.
    pub const CREATURE: Self = Self::HasType(TypeSet::CREATURE);
    /// An artifact.
    pub const ARTIFACT: Self = Self::HasType(TypeSet::ARTIFACT);
    /// An enchantment.
    pub const ENCHANTMENT: Self = Self::HasType(TypeSet::ENCHANTMENT);
    /// A land.
    pub const LAND: Self = Self::HasType(TypeSet::LAND);
    /// A planeswalker.
    pub const PLANESWALKER: Self = Self::HasType(TypeSet::PLANESWALKER);
    /// Anything that is not a land — "nonland permanent".
    pub const NONLAND: Self = Self::LacksType(TypeSet::LAND);
    /// Anything that is not a creature.
    pub const NONCREATURE: Self = Self::LacksType(TypeSet::CREATURE);
    /// A basic land — the *supertype* Basic plus the land type (CR 205.4a),
    /// which is why it is two clauses and not a type check.
    ///
    /// The clause order is the one the pool already used, so swapping a
    /// hand-written filter for this constant is provably the same data and
    /// not merely the same meaning.
    pub const BASIC_LAND: Self = Self::And(&[Self::HasSupertype(SupertypeSet::BASIC), Self::LAND]);
    /// An instant or a sorcery.
    pub const INSTANT_OR_SORCERY: Self = Self::Or(&[
        Self::HasType(TypeSet::INSTANT),
        Self::HasType(TypeSet::SORCERY),
    ]);
    /// An artifact or an enchantment.
    pub const ARTIFACT_OR_ENCHANTMENT: Self = Self::Or(&[Self::ARTIFACT, Self::ENCHANTMENT]);
    /// An artifact or a creature — what a clone copies and what half the
    /// removal in this pool points at.
    pub const ARTIFACT_OR_CREATURE: Self = Self::Or(&[Self::ARTIFACT, Self::CREATURE]);
    /// An artifact, a creature, or an enchantment.
    pub const ARTIFACT_CREATURE_OR_ENCHANTMENT: Self =
        Self::Or(&[Self::ARTIFACT, Self::CREATURE, Self::ENCHANTMENT]);
    /// A creature or a planeswalker — the two things damage and destruction
    /// point at, and the most-written filter in the pool that had no name:
    /// four writings in three card files, one of which wrote it twice.
    pub const CREATURE_OR_PLANESWALKER: Self = Self::Or(&[Self::CREATURE, Self::PLANESWALKER]);
    /// A land that is not basic.
    ///
    /// `Not(&HasSupertype(BASIC))` and not a `LacksSupertype`, because that
    /// is the spelling both writings already used and the one the script
    /// reader builds from `Land.nonBasic`.
    pub const NONBASIC_LAND: Self = Self::And(&[
        Self::LAND,
        Self::Not(&Self::HasSupertype(SupertypeSet::BASIC)),
    ]);
    /// A creature that is not a token.
    pub const NONTOKEN_CREATURE: Self = Self::And(&[Self::CREATURE, Self::Not(&Self::IsToken)]);
    /// A creature other than the source ("another creature").
    pub const ANOTHER_CREATURE: Self = Self::And(&[Self::CREATURE, Self::Another]);
    /// A legendary creature.
    ///
    /// Noun first, like its neighbours and like [`f!`](crate::f) — four card
    /// files had written this out, three of them generated, and the order
    /// they all used is the one kept here so that the swap is provably the
    /// same data.
    pub const LEGENDARY_CREATURE: Self =
        Self::And(&[Self::CREATURE, Self::HasSupertype(SupertypeSet::LEGENDARY)]);
    /// A creature that is attacking.
    pub const ATTACKING_CREATURE: Self = Self::And(&[Self::CREATURE, Self::Attacking]);
    /// A creature you control.
    pub const YOUR_CREATURE: Self = Self::And(&[Self::CREATURE, Self::ControlledByYou]);
    /// A creature an opponent controls.
    pub const OPPONENT_CREATURE: Self = Self::And(&[Self::CREATURE, Self::ControlledByOpponent]);
    /// A land you control.
    ///
    /// Noun first, and here that is a decision rather than a habit: the pool
    /// wrote this one adjective first, so the byte-identical spelling would
    /// have been `And(&[ControlledByYou, LAND])` — and that is the one order
    /// this constant may not have, because [`f!`](crate::f) expands
    /// `f!(your LAND)` noun first and a constant the macro cannot spell is a
    /// filter with two names again. So the clauses moved instead: ten slow
    /// lands count this one and the commit that reordered them names every
    /// card whose dump changed.
    pub const YOUR_LAND: Self = Self::And(&[Self::LAND, Self::ControlledByYou]);
    /// A basic land you control.
    ///
    /// Noun first for the reason [`Self::YOUR_LAND`] gives; the noun is
    /// [`Self::BASIC_LAND`], so this nests rather than listing three
    /// clauses, which is what `f!(your BASIC_LAND)` builds — and is why the
    /// ten battle lands that count it are the only cards that may use it.
    /// A card that flattens the three clauses is a different filter and
    /// keeps its own.
    pub const YOUR_BASIC_LAND: Self = Self::And(&[Self::BASIC_LAND, Self::ControlledByYou]);
    /// An artifact you control.
    pub const YOUR_ARTIFACT: Self = Self::And(&[Self::ARTIFACT, Self::ControlledByYou]);
    /// A creature you control other than the source — "another creature you
    /// control".
    ///
    /// Spelled out rather than shortened to `ANOTHER_YOUR_CREATURE`, because
    /// [`Self::ANOTHER_CREATURE`] is already taken and means something else:
    /// a creature other than the source, *whoever* controls it. The two
    /// differ by one clause and by every card that reads them, so the longer
    /// name is the cheap half of that bargain.
    ///
    /// The clause order is `your` before `another`, which matches
    /// `ANOTHER_ALLY` in `baylee-cards`. It has to be *a* choice rather than
    /// the obvious one, because this is the first constant whose English
    /// [`f!`](crate::f) can spell two ways: `f!(your another CREATURE)` is
    /// this, and `f!(another your CREATURE)` is the same objects in a
    /// different order — a different `Filter` and a different hash.
    /// `the_filter_macro_cannot_be_trusted_to_spell_an_order` in `build.rs`
    /// is that fact as a build failure.
    pub const ANOTHER_CREATURE_YOU_CONTROL: Self =
        Self::And(&[Self::CREATURE, Self::ControlledByYou, Self::Another]);
}
