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
    /// Has at least one of these types.
    ///
    /// *One* of them, not all: `eval` asks `types.intersects(t)`, so
    /// `HasType(INSTANT.union(SORCERY))` is "an instant or a sorcery" and
    /// not "both at once", which nothing is. Every writing in this pool
    /// names a single type, so the doc that said "all" had never been wrong
    /// about a card — it was wrong about the one thing
    /// [`Self::LacksType`] is the negation of.
    HasType(TypeSet),
    /// Has none of these types — the exact negation of [`Self::HasType`],
    /// and therefore the one spelling for "not a creature". `Not(&HasType(t))`
    /// matches the same objects and is a different `Filter`: only
    /// `state::filter_hash` can tell them apart, and a filter with two
    /// spellings is a filter with two names.
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
    /// An object whose **name** is this one (Muscle Burst counting copies of
    /// itself in every graveyard).
    ///
    /// The name is a rules characteristic and not a handle, so this is a
    /// `&str` and not a `CardIndex`: CR 201.2 compares what an object is
    /// *called*, so a clone, a face-down turned face up and a card whose
    /// name a text-changing effect has rewritten all answer by the name they
    /// carry now. `docs/card-identity.md` is normative on which handle may
    /// be stored where, and a name is the one that may not.
    Named(&'static str),
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
    /// Entered the battlefield during the current turn (Oran-Rief, the
    /// Vastwood; Ruins of Oran-Rief; Novijen, Heart of Progress).
    ///
    /// History rather than a characteristic: nothing on the object says it.
    /// The engine keeps the turn's arrivals in a per-turn record, beside
    /// the flag `Effect::IfNotLostLifeThisTurn` reads for "lost life this
    /// turn".
    ///
    /// A view cannot answer it at all: a `PlayerView` carries no such
    /// record, so `baylee-ai` and the client say "don't know" and take the
    /// engine's enumerated options instead, which is what they already do
    /// for every filter they cannot read.
    EnteredThisTurn,
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
    /// Mana value at most **X**, the value announced for the ability's own
    /// source (CR 107.3a).
    ///
    /// Not [`Self::CmcAtMost`] with a number in it, and that is the whole
    /// reason it exists: a card printing "a creature card with mana value X
    /// or less" does not know the bound until it is cast, so every such card
    /// was inexpressible. The number is read off the source object rather
    /// than threaded through the matcher, because that is where the engine
    /// already writes it — a filter is evaluated with the ability's source
    /// in hand and with no announced value beside it.
    ///
    /// An ability that announces no X reads 0, which is what
    /// [`crate::Amount::X`] does in the same position: a triggered ability
    /// has no announcement to read.
    CmcAtMostX,
    /// Converted mana cost at least N.
    CmcAtLeast(u32),
    /// Toughness at most N (Recruiter of the Guard).
    ToughnessAtMost(i16),
    /// Toughness at least N (Baxter Building's draw, gated on "a creature
    /// with toughness 4 or greater").
    ToughnessAtLeast(i16),
    /// Power at least N (Bonders' Enclave, Garruk's Uprising, Temur
    /// Ascendancy).
    ///
    /// The power twin of the two above, and its absence was not one card
    /// sitting unwritten: six cards in this pool print a power comparison
    /// and every one of them had the ability that states it taken **off**
    /// the card, because the alternative — offering `{3}, {T}: Draw a card`
    /// with no gate at all — is a land strictly stronger than the printed
    /// one. CR 602.5 forbids beginning an activation that is prohibited, so
    /// a gate that cannot be stated is a gate that must not be approximated.
    ///
    /// It reads the **projected** power, like every other predicate here: a
    /// 1/1 under an anthem that makes it 4/4 is a creature with power 4 or
    /// greater, which is what CR 613 makes it and what a player sees.
    PowerAtLeast(i16),
    /// Power at most N (Access Tunnel, Escape Tunnel — "target creature with
    /// power 3 or less can't be blocked this turn").
    PowerAtMost(i16),
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Every constant this file declares, beside the name it is declared
    /// under.
    ///
    /// Hand-written because what is compared here is the *value* and no
    /// reader of the text can supply one — and held against the
    /// declarations below, so a constant added tomorrow fails the check
    /// rather than being quietly left out of every comparison in this
    /// module.
    const NAMED: &[(&str, Filter)] = &[
        ("CREATURE", Filter::CREATURE),
        ("ARTIFACT", Filter::ARTIFACT),
        ("ENCHANTMENT", Filter::ENCHANTMENT),
        ("LAND", Filter::LAND),
        ("PLANESWALKER", Filter::PLANESWALKER),
        ("NONLAND", Filter::NONLAND),
        ("NONCREATURE", Filter::NONCREATURE),
        ("BASIC_LAND", Filter::BASIC_LAND),
        ("INSTANT_OR_SORCERY", Filter::INSTANT_OR_SORCERY),
        ("ARTIFACT_OR_ENCHANTMENT", Filter::ARTIFACT_OR_ENCHANTMENT),
        ("ARTIFACT_OR_CREATURE", Filter::ARTIFACT_OR_CREATURE),
        (
            "ARTIFACT_CREATURE_OR_ENCHANTMENT",
            Filter::ARTIFACT_CREATURE_OR_ENCHANTMENT,
        ),
        ("CREATURE_OR_PLANESWALKER", Filter::CREATURE_OR_PLANESWALKER),
        ("NONBASIC_LAND", Filter::NONBASIC_LAND),
        ("NONTOKEN_CREATURE", Filter::NONTOKEN_CREATURE),
        ("ANOTHER_CREATURE", Filter::ANOTHER_CREATURE),
        ("LEGENDARY_CREATURE", Filter::LEGENDARY_CREATURE),
        ("ATTACKING_CREATURE", Filter::ATTACKING_CREATURE),
        ("YOUR_CREATURE", Filter::YOUR_CREATURE),
        ("OPPONENT_CREATURE", Filter::OPPONENT_CREATURE),
        ("YOUR_LAND", Filter::YOUR_LAND),
        ("YOUR_BASIC_LAND", Filter::YOUR_BASIC_LAND),
        ("YOUR_ARTIFACT", Filter::YOUR_ARTIFACT),
        (
            "ANOTHER_CREATURE_YOU_CONTROL",
            Filter::ANOTHER_CREATURE_YOU_CONTROL,
        ),
    ];

    /// The table above is the file below. Read out of the source for the
    /// reason `the_authoring_contract_names_every_filter_constant` reads it:
    /// a list retyped and left alone is the same defect the constants exist
    /// to end, one layer up.
    #[test]
    fn the_table_is_every_constant_this_file_declares() {
        let declared: Vec<&str> = include_str!("filter.rs")
            .lines()
            .filter_map(|line| line.trim().strip_prefix("pub const "))
            .filter_map(|rest| rest.split(':').next())
            .map(str::trim)
            .collect();
        let listed: Vec<&str> = NAMED.iter().map(|(name, _)| *name).collect();
        assert_eq!(
            declared, listed,
            "the table and the declarations have drifted — in this order, \
             because a constant belongs beside the ones it is built from"
        );
    }

    /// **No filter has two names.** Two constants that are the same data are
    /// the sin this module's head describes from the other side: `eval`
    /// cannot tell them apart, `state::filter_hash` gives them one hash, and
    /// a card author picking between them is choosing a synonym while
    /// believing they chose a meaning.
    ///
    /// It is a real hazard rather than a tidy one, because these are built
    /// out of each other: `NONCREATURE` and `NONLAND` differ by one
    /// `TypeSet` constant, and `ARTIFACT_OR_CREATURE` and
    /// `ARTIFACT_OR_ENCHANTMENT` by one clause.
    #[test]
    fn no_two_constants_are_the_same_filter() {
        for (i, (name, filter)) in NAMED.iter().enumerate() {
            for (other, twin) in &NAMED[i + 1..] {
                assert_ne!(
                    filter, twin,
                    "{name} and {other} are one filter under two names"
                );
            }
        }
        assert!(NAMED.len() >= 24, "read {} constants", NAMED.len());
    }

    /// **The same objects are not the same filter**, which is the sentence
    /// [`Filter::LacksType`] is documented with and the reason it exists at
    /// all. Each pair below matches exactly the same objects in every game
    /// state there is, and each is two distinct pieces of data — so a card
    /// that writes the wrong one is correct in play and a second name for a
    /// filter that already had one.
    #[test]
    fn two_spellings_of_one_predicate_are_two_filters() {
        static NOT_A_CREATURE: Filter = Filter::HasType(TypeSet::CREATURE);
        assert_ne!(
            Filter::NONCREATURE,
            Filter::Not(&NOT_A_CREATURE),
            "`LacksType(t)` is the one spelling for \"not a creature\""
        );

        assert_ne!(
            Filter::INSTANT_OR_SORCERY,
            Filter::HasType(TypeSet::INSTANT.union(TypeSet::SORCERY)),
            "`HasType` asks `intersects`, so a two-type set is already an \
             \"or\" — and it is not the `Or` the pool writes"
        );

        assert_ne!(
            Filter::YOUR_LAND,
            Filter::And(&[Filter::ControlledByYou, Filter::LAND]),
            "the same two clauses the other way round"
        );

        assert_ne!(
            Filter::YOUR_BASIC_LAND,
            Filter::And(&[
                Filter::HasSupertype(SupertypeSet::BASIC),
                Filter::LAND,
                Filter::ControlledByYou,
            ]),
            "and `YOUR_BASIC_LAND` nests its noun rather than flattening it"
        );
    }

    /// The clause order of every compound constant, which is data and not
    /// style: `f!` spells one order, `state::filter_hash` hashes the order,
    /// and the commit that introduced each of these proved it byte-identical
    /// to what the cards already wrote. Noun first throughout, with the
    /// nesting `YOUR_BASIC_LAND` documents.
    #[test]
    fn every_compound_constant_keeps_the_order_it_was_proved_against() {
        assert_eq!(
            Filter::BASIC_LAND,
            Filter::And(&[Filter::HasSupertype(SupertypeSet::BASIC), Filter::LAND]),
            "the supertype first, which is the order CR 205.4a reads in too"
        );
        assert_eq!(
            Filter::YOUR_LAND,
            Filter::And(&[Filter::LAND, Filter::ControlledByYou])
        );
        assert_eq!(
            Filter::YOUR_BASIC_LAND,
            Filter::And(&[Filter::BASIC_LAND, Filter::ControlledByYou])
        );
        assert_eq!(
            Filter::ANOTHER_CREATURE_YOU_CONTROL,
            Filter::And(&[Filter::CREATURE, Filter::ControlledByYou, Filter::Another]),
            "your before another — the one order `f!` can spell two ways"
        );
        assert_eq!(
            Filter::NONBASIC_LAND,
            Filter::And(&[
                Filter::LAND,
                Filter::Not(&Filter::HasSupertype(SupertypeSet::BASIC)),
            ]),
            "`Not(HasSupertype)` and not a `LacksSupertype` that does not exist"
        );
        assert_eq!(
            Filter::ARTIFACT_CREATURE_OR_ENCHANTMENT,
            Filter::Or(&[Filter::ARTIFACT, Filter::CREATURE, Filter::ENCHANTMENT]),
            "and an `Or` is written in the order the card prints the words"
        );
    }
}
