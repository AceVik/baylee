//! baylee-cards-dsl — the card authoring framework.
//!
//! M0 defines the compiled data model ([`CardDef`]); the ability/effect
//! vocabulary ([`AbilityDef`]) is filled during M1–M2 and frozen as the LLM
//! authoring contract (`docs/card-dsl.md`).

#![warn(missing_docs)]

pub mod ability;
pub mod cost;
pub mod effect;
pub mod filter;
pub mod static_ability;

pub use ability::{
    AbilityDef, ActivationCondition, ActivationTiming, ActivationZone, CopyMod, SpellMode,
    StepKind, Trigger, TriggerEventKind,
};
pub use cost::{AltCondition, AlternativeCost, Cost, CostPart, CostReduction};
pub use effect::{
    Amount, CounterKind, Effect, PlayerRel, SearchDest, SpendRider, TargetReq, TargetSpec,
    TokenDef, ZoneSel,
};
pub use filter::{Filter, ZoneRef};
pub use static_ability::{Duration, LAYERS, Layer, Modifier, ReplacementRule, StaticAbility};

use baylee_core::color::ColorSet;
use baylee_core::ids::{CardIndex, SubtypeId};
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

/// A compiled card definition: zero-cost, `'static`, registry-resident.
#[derive(Debug)]
pub struct CardDef {
    /// Dense runtime index (fast registry lookups).
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
    /// Whether this face can be cast from the hand (false for disturb
    /// backs — they are cast from the graveyard instead).
    pub castable_from_hand: bool,
    /// Miracle cost: when revealed as the first card drawn this turn, the
    /// card may be cast for this cost (CR 702.94).
    pub miracle: Option<ManaCost>,
    /// Delve (CR 702.66): each card exiled from your graveyard while
    /// casting pays for {1}.
    pub delve: bool,
    /// Convoke (CR 702.51): each creature tapped while casting pays for
    /// {1} (colored-mana option is a payment refinement).
    pub convoke: bool,
    /// A conditional cost reduction printed on the card.
    pub cost_reduction: Option<crate::cost::CostReduction>,
    /// Disturb (CR 702.112): this face may be cast from the graveyard
    /// for its mana cost; exile it after.
    pub disturb: bool,
    /// Adventure (CR 715): this face is an Adventure spell — when it
    /// resolves, exile the card; the front face may then be cast from
    /// exile.
    pub adventure: bool,
}

/// As-it-enters-the-battlefield modifiers (CR 614.1c/d).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum EnterModifier {
    /// Enters tapped (triomes, Arcane Sanctum).
    Tapped,
    /// Enters tapped unless you control a matching permanent (checklands).
    TappedUnless(&'static Filter),
    /// "You may pay N life; if you don't, this enters tapped" (shocklands).
    TappedOrPayLife(u16),
    /// "As this enters, choose a creature type" (Roaming Throne,
    /// Reflections of Littjara, Cavern of Souls).
    ChooseSubtype,
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
    /// "Partner with <name>".
    PartnerWith(&'static str),
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
