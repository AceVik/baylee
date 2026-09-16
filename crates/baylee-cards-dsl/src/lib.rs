//! baylee-cards-dsl — the card authoring framework.
//!
//! M0 defines the compiled data model ([`CardDef`]); the ability/effect
//! vocabulary ([`AbilityDef`]) is filled during M1–M2 and frozen as the LLM
//! authoring contract (`docs/card-dsl.md`).

#![warn(missing_docs)]

pub mod ability;
pub mod build;
pub mod cost;
pub mod effect;
pub mod filter;
pub mod manaread;
pub mod static_ability;

/// All five mana colors (choice-mana abilities: Cavern of Souls, City
/// of Brass, Moxen, …).
pub const ALL_MANA_COLORS: &[baylee_core::mana::ManaColor] = &[
    baylee_core::mana::ManaColor::White,
    baylee_core::mana::ManaColor::Blue,
    baylee_core::mana::ManaColor::Black,
    baylee_core::mana::ManaColor::Red,
    baylee_core::mana::ManaColor::Green,
];

/// The reusable "add one mana of any color" effect (Chromatic Lantern,
/// City of Brass, Great Divide Guide).
pub static ANY_COLOR_MANA: &[crate::effect::Effect] = &[crate::effect::Effect::mana_of_any_color()];

pub use ability::{
    AbilityDef, ActivationCondition, ActivationTiming, ActivationZone, CopyMod, SpellMode,
    StepKind, Trigger, TriggerEventKind,
};
pub use build::prelude;
pub use build::{
    ActivatedParts, EQUIP_TARGET, LoyaltyParts, ModalTriggeredParts, SagaChapterParts, SpellParts,
    StaticParts, TriggeredParts,
};
pub use cost::{AltCondition, AlternativeCost, Cost, CostPart, CostReduction};
pub use effect::{
    Amount, CounterKind, Effect, Find, ManaRestriction, ManaSource, PlayerRel, SearchDest,
    SpendRider, TargetReq, TargetSpec, TokenDef, ZoneSel,
};
pub use filter::{Filter, ZoneRef};
pub use manaread::{SimpleMana, mana_made, mana_offer, mana_shape, simple_mana};
// Re-exported so the authoring macros can name them through `$crate`, and so
// a card file needs exactly one import (see `build::prelude`).
pub use baylee_core::color::{Color, ColorSet};
pub use baylee_core::ids::{CardIndex, SubtypeId};
pub use baylee_core::mana;
pub use baylee_core::mana::{ManaColor, ManaCost};
pub use baylee_core::types::{SupertypeSet, TypeSet};
pub use static_ability::{Duration, LAYERS, Layer, Modifier, ReplacementRule, StaticAbility};

/// A compiled card definition: zero-cost, `'static`, registry-resident.
#[derive(Debug)]
pub struct CardDef {
    /// Rules identity, assigned once over the whole card corpus and never
    /// reused (`docs/card-identity.md`). Not a position: most indices name a
    /// card this repo compiles no `CardDef` for.
    pub index: CardIndex,
    /// Scryfall oracle id — rules identity shared by all printings.
    pub oracle_id: &'static str,
    /// Scryfall id of the reference printing used by codegen.
    pub scryfall_id: &'static str,
    /// Faces: one for normal cards, two for MDFC/split/adventure/…
    pub faces: &'static [FaceDef],
    /// Color identity across all faces (deckbuilding rule, CR 903.4).
    pub color_identity: ColorSet,
    /// Simple keyword abilities printed on the card.
    pub keywords: KeywordSet,
    /// Commander eligibility.
    pub commander: CommanderRule,
    /// Partner-family membership.
    pub partner: PartnerKind,
    /// Implementation coverage (surfaced by the gateway deckbuilder).
    pub coverage: Coverage,
    /// Ability definitions; empty until the card is implemented.
    pub abilities: &'static [AbilityDef],
}

impl CardDef {
    /// Name of the front face.
    #[must_use]
    pub fn name(&self) -> &'static str {
        self.faces.first().map_or("<unnamed>", |f| f.name)
    }

    /// Whether the card is rules-complete.
    #[must_use]
    pub const fn is_implemented(&self) -> bool {
        matches!(self.coverage, Coverage::Implemented)
    }

    /// Abilities of a face. Face 0 uses the face's own list when
    /// non-empty, else the card-level list (single-face convention).
    /// Back faces (MDFC) use ONLY their own list — they never inherit
    /// the front's abilities.
    #[must_use]
    pub fn abilities_for_face(&self, face: usize) -> &'static [crate::ability::AbilityDef] {
        if face == 0 {
            let face_abilities = self.faces[0].abilities;
            if face_abilities.is_empty() {
                self.abilities
            } else {
                face_abilities
            }
        } else {
            self.faces[face.min(self.faces.len() - 1)].abilities
        }
    }

    /// Keywords of a face, with [`Self::abilities_for_face`]'s rule: face 0
    /// falls back to the card-level set when it states none of its own, a
    /// back face uses only what it prints.
    ///
    /// The fallback is what keeps every single-faced card writing its
    /// keywords once. The *absence* of a fallback on the back is the load-
    /// bearing half: a front face with daybound and a back face with
    /// nightbound are opposite faces of one card (CR 702.145a), and a card
    /// whose keywords were one set would answer "has daybound" for the
    /// night side too — so CR 702.145g's "no permanents with daybound on
    /// the battlefield" could never be true and CR 702.145c would turn a
    /// permanent that is already turned over.
    #[must_use]
    pub fn keywords_for_face(&self, face: usize) -> KeywordSet {
        if face == 0 {
            let own = self.faces.first().map_or(KeywordSet::EMPTY, |f| f.keywords);
            if own.is_empty() { self.keywords } else { own }
        } else {
            self.faces[face.min(self.faces.len() - 1)].keywords
        }
    }

    /// Every keyword printed anywhere on the card.
    ///
    /// For the checks that ask about the card rather than the permanent —
    /// "does any rule read this bit" — where reading one face would miss
    /// what the other prints.
    #[must_use]
    pub fn all_keywords(&self) -> KeywordSet {
        (0..self.faces.len().max(1))
            .map(|f| self.keywords_for_face(f))
            .fold(self.keywords, KeywordSet::union)
    }
}

impl CardDef {
    /// The neutral card: no faces, no abilities, not implemented.
    ///
    /// Card files spell out only what distinguishes the card and finish with
    /// `..CardDef::DEFAULT`. Two defaults are deliberately the *pessimistic*
    /// choice rather than the common one:
    ///
    /// - `index` is 0, which every other card's index collides with. The
    ///   registry consistency test in `baylee-cards` fails loudly if a card
    ///   file forgets it, instead of silently shadowing card 0.
    /// - `coverage` is [`Coverage::Unimplemented`], so a stub that was never
    ///   finished cannot claim to be playable in the deckbuilder just because
    ///   somebody deleted a line.
    pub const DEFAULT: Self = Self {
        index: CardIndex::new(0),
        oracle_id: "",
        scryfall_id: "",
        faces: &[],
        color_identity: ColorSet::EMPTY,
        keywords: KeywordSet::EMPTY,
        commander: CommanderRule::NotEligible,
        partner: PartnerKind::None,
        coverage: Coverage::Unimplemented,
        abilities: &[],
    };
}

impl Default for CardDef {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// One face of a card.
#[derive(Debug)]
#[allow(clippy::struct_excessive_bools)] // card faces accumulate boolean rule markers
pub struct FaceDef {
    /// Face name.
    pub name: &'static str,
    /// Mana cost (`ManaCost::ZERO` for lands/MDFC backs without cost).
    pub mana_cost: ManaCost,
    /// Types.
    pub types: TypeSet,
    /// Supertypes.
    pub supertypes: SupertypeSet,
    /// Subtypes (definition-side list; instances use the 512-bit bitmap).
    pub subtypes: &'static [SubtypeId],
    /// Power (creatures).
    pub power: Option<i16>,
    /// Toughness (creatures).
    pub toughness: Option<i16>,
    /// Loyalty (planeswalkers).
    pub loyalty: Option<u16>,
    /// Alternative costs (pitch, evoke, conditional free — CR 601.2b).
    pub alternative_costs: &'static [crate::cost::AlternativeCost],
    /// Optional additional costs offered at cast (kicker, CR 702.33).
    pub additional_costs: &'static [crate::cost::Cost],
    /// Mandatory additional cost parts paid at cast (Toxic Deluge's
    /// "pay X life").
    pub mandatory_additional_costs: &'static [crate::cost::CostPart],
    /// As-it-enters-the-battlefield modifiers (taplands, shocklands).
    pub enter_modifiers: &'static [EnterModifier],
    /// Per-face abilities (MDFC backs; face 0 falls back to the
    /// card-level list when empty).
    pub abilities: &'static [crate::ability::AbilityDef],
    /// Simple keywords printed on *this* face, with the same fallback as
    /// [`FaceDef::abilities`] — see [`CardDef::keywords_for_face`].
    ///
    /// A single-faced card leaves this empty and states its keywords once,
    /// at card level. A transforming card cannot: daybound is printed on
    /// the front face and nightbound on the back (CR 702.145a), and a card
    /// claiming both at once would be a permanent that is simultaneously
    /// the thing that turns over at night and the thing that turns back.
    pub keywords: KeywordSet,
    /// Color indicator (CR 202.2e): the dot printed on a face with no mana
    /// cost, which is where its color comes from.
    ///
    /// Empty on every face that has a cost — the cost already says it. It
    /// exists for transformed backs: Dire-Strain Brawler is green, and
    /// nothing but this says so, so without it every werewolf stopped being
    /// green the moment it turned over.
    pub color_indicator: ColorSet,
    /// Whether this face can be cast from the hand (false for disturb
    /// backs — they are cast from the graveyard instead).
    pub castable_from_hand: bool,
    /// Miracle cost: when revealed as the first card drawn this turn, the
    /// card may be cast for this cost (CR 702.94).
    pub miracle: Option<ManaCost>,
    /// Delve (CR 702.66): each card exiled from your graveyard while
    /// casting pays for {1} — as many cards as the spell's total cost has
    /// generic mana, and no more (CR 702.66a).
    pub delve: bool,
    /// Convoke (CR 702.51): each creature tapped while casting pays for
    /// {1} (colored-mana option is a payment refinement).
    pub convoke: bool,
    /// A conditional cost reduction printed on the card.
    pub cost_reduction: Option<crate::cost::CostReduction>,
    /// Disturb (CR 702.146): this face may be cast from the graveyard
    /// for its mana cost; exile it after.
    pub disturb: bool,
    /// Adventure (CR 715): this face is an Adventure spell — when it
    /// resolves, exile the card; the front face may then be cast from
    /// exile.
    pub adventure: bool,
}

impl FaceDef {
    /// The neutral face: nameless, costless, typeless, castable from hand.
    ///
    /// Every card file builds its faces as `FaceDef { …, ..FaceDef::DEFAULT }`
    /// so that a face lists only what is printed on it. Adding a field to
    /// [`FaceDef`] then costs one line here instead of one line in every card
    /// file, and a card that does not care about the new rule keeps compiling.
    ///
    /// `castable_from_hand` defaults to `true` because that is what a printed
    /// face normally is; only disturb/adventure backs opt out.
    pub const DEFAULT: Self = Self {
        name: "",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::EMPTY,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
        abilities: &[],
        keywords: KeywordSet::EMPTY,
        color_indicator: ColorSet::EMPTY,
        castable_from_hand: true,
        miracle: None,
        delve: false,
        convoke: false,
        cost_reduction: None,
        disturb: false,
        adventure: false,
    };
}

impl Default for FaceDef {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// As-it-enters-the-battlefield modifiers (CR 614.1c/d).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum EnterModifier {
    /// Enters tapped (triomes, Arcane Sanctum).
    Tapped,
    /// Enters tapped unless you control a matching permanent (checklands).
    TappedUnless(&'static Filter),
    /// Enters tapped unless you control at least `at_least` matching
    /// permanents (the battle lands' "two or more basic lands").
    ///
    /// A variant of its own rather than a count on [`Self::TappedUnless`]: a
    /// checkland asks about *a* permanent, which is what its sentence says,
    /// and a card never restates a default.
    TappedUnlessCount {
        /// What each of them has to be.
        filter: &'static Filter,
        /// How many of them it takes.
        at_least: u8,
    },
    /// "You may pay N life; if you don't, this enters tapped" (shocklands).
    TappedOrPayLife(u16),
    /// "As this enters, choose a creature type" (Roaming Throne,
    /// Reflections of Littjara, Cavern of Souls).
    ChooseSubtype,
    /// Enters with the prepared marker (Emeritus of Woe).
    Prepared,
    /// "This enters with N [kind] counters on it" (the Vivid lands, Tendo
    /// Ice Bridge, the storage lands, Wishclaw Talisman).
    ///
    /// A replacement effect like every other modifier here (CR 614.1c), and
    /// that is the load-bearing part rather than a citation: a counter
    /// doubler applies to what a replacement effect places (CR 614.16), so
    /// a Vivid land arriving under a Doubling Season brings four charge
    /// counters and not two. The engine therefore puts them through
    /// `replacement::put_counters`, the same door a resolving spell uses,
    /// and inherits that for nothing.
    ///
    /// **Not** the same thing as a counter paid as a cost, which takes the
    /// other door on purpose — see [`crate::CostPart::RemoveCounterSelf`].
    WithCounters {
        /// Which counter.
        kind: crate::effect::CounterKind,
        /// How many, before any replacement multiplies them.
        n: u16,
    },
}

/// Simple keyword abilities as a bitset.
///
/// Parameterized keywords (equip {2}, crew N, kicker, ward {2}, …) are NOT
/// bits — they are [`AbilityDef`] data. Only text-independent keywords live
/// here.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
pub struct KeywordSet(u128);

macro_rules! keywords {
    ($($name:ident = $bit:expr, $doc:literal;)*) => {
        impl KeywordSet {
            $(#[doc = $doc] pub const $name: Self = Self(1 << $bit);)*
        }
    };
}

keywords! {
    FLYING = 0, "Flying.";
    FIRST_STRIKE = 1, "First strike.";
    DOUBLE_STRIKE = 2, "Double strike.";
    DEATHTOUCH = 3, "Deathtouch.";
    HASTE = 4, "Haste.";
    HEXPROOF = 5, "Hexproof.";
    INDESTRUCTIBLE = 6, "Indestructible.";
    LIFELINK = 7, "Lifelink.";
    MENACE = 8, "Menace.";
    REACH = 9, "Reach.";
    TRAMPLE = 10, "Trample.";
    VIGILANCE = 11, "Vigilance.";
    DEFENDER = 12, "Defender.";
    FLASH = 13, "Flash.";
    SHROUD = 14, "Shroud.";
    FEAR = 15, "Fear.";
    INTIMIDATE = 16, "Intimidate.";
    SHADOW = 17, "Shadow.";
    HORSEMANSHIP = 18, "Horsemanship.";
    INFECT = 19, "Infect.";
    WITHER = 20, "Wither.";
    PERSIST = 21, "Persist.";
    UNDYING = 22, "Undying.";
    PROWESS = 23, "Prowess.";
    SKULK = 24, "Skulk.";
    FLANKING = 25, "Flanking.";
    CHANGELING = 26, "Changeling (every creature type).";
    PARTNER = 27, "Partner (generic).";
    UNBLOCKABLE = 28, "Can't be blocked.";
    UNCOUNTERABLE = 29, "Can't be countered.";
    REBOUND = 30, "Rebound.";
    PROTECTION_BLACK = 31, "Protection from black.";
    DAYBOUND = 32, "Daybound (front faces only, CR 702.145b).";
    NIGHTBOUND = 33, "Nightbound (back faces only, CR 702.145e).";
}

impl KeywordSet {
    /// No keywords.
    pub const EMPTY: Self = Self(0);

    /// Raw bits.
    #[must_use]
    pub const fn bits(self) -> u128 {
        self.0
    }

    /// Whether the keyword is present.
    #[inline]
    #[must_use]
    pub const fn contains(self, k: Self) -> bool {
        self.0 & k.0 != 0
    }

    /// Union.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Difference (keyword removal).
    #[must_use]
    pub const fn difference(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }

    /// Whether no keywords are set.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

/// Commander eligibility, derived from oracle data at codegen time.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum CommanderRule {
    /// Cannot be a commander.
    NotEligible,
    /// Eligible as a legendary creature.
    Legendary,
    /// Eligible because the oracle text says so ("can be your commander").
    ExplicitlyAllowed,
}

/// Partner-family membership (CR 702.124).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum PartnerKind {
    /// No partner ability.
    None,
    /// "Partner" (pairs with any other generic Partner).
    Partner,
    /// "Partner with <name>", carrying the named card's [`CardIndex`].
    ///
    /// The printed sentence names a card by its English name and this does
    /// not, for three reasons. The ledger numbers every card there is, so
    /// `index::TOOTHY_IMAGINARY_FRIEND` resolves whether or not this repo
    /// compiles Toothy — a name would have had to survive the same trip
    /// unchecked. A misspelled name compiles and then pairs with nothing,
    /// in silence, for as long as nobody plays the pair; a misspelled
    /// constant does not compile. And a name reaches the deckbuilder as a
    /// second thing to match: a `PoolCard` already carries `index`, so
    /// "may these two lead together" is one integer compare rather than a
    /// name comparison that would have to pick between the card's whole
    /// name (`Sheoldred // The True Scriptures`, which is what the ledger
    /// stores) and its front face (`Sheoldred`, which is what the pool
    /// stores and what `Partner with` prints).
    PartnerWith(CardIndex),
    /// "Choose a Background".
    ChooseABackground,
    /// "Friends forever".
    FriendsForever,
    /// "Doctor's companion".
    DoctorsCompanion,
}

/// Implementation coverage of a card (shown in the deckbuilder).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Coverage {
    /// Rules-complete, tested.
    Implemented,
    /// Partially implemented; note describes the gap.
    Partial(&'static str),
    /// Stub only.
    Unimplemented,
}
