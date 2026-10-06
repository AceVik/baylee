//! Counter kinds, and the tokens an effect makes.

use super::{ColorSet, KeywordSet, SubtypeId, SupertypeSet, TypeSet};

/// Counter kinds (objects and players). Lives here so card definitions can
/// reference counters without engine dependencies.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
pub enum CounterKind {
    /// A +X/+Y counter (CR 122.1a): it adds X to power and Y to toughness.
    ///
    /// One variant rather than a name per printed pair, because the rule is
    /// one rule and the pairs are open-ended — the reference corpus prints
    /// eleven of them (`P1P1`, `P1P0`, `P2P2`, `P0P1`, `P1P2`, and the
    /// mirrors), and a table of names would go silent on the twelfth.
    /// [`Self::P1P1`] is the const for the one Magic prints everywhere.
    Plus {
        /// X.
        power: u8,
        /// Y.
        toughness: u8,
    },
    /// A −X/−Y counter (CR 122.1a): it subtracts. The mirror of
    /// [`Self::Plus`], and a separate variant rather than one signed pair
    /// because the sign is part of what the counter *is*: CR 122.1a names
    /// exactly these two forms, CR 704.5q annihilates the +1/+1 against the
    /// −1/−1 and nothing else, and a card that counts its −0/−1 counters
    /// must not be answered with +0/−1 ones. Every P/T code the reference
    /// corpus prints is single-signed, which is the same fact measured from
    /// the other side.
    Minus {
        /// X.
        power: u8,
        /// Y.
        toughness: u8,
    },
    /// Loyalty.
    Loyalty,
    /// Lore (sagas).
    Lore,
    /// Time (suspend, vanishing).
    Time,
    /// Charge.
    Charge,
    /// Poison (players).
    Poison,
    /// Energy (players).
    Energy,
    /// Rad (players).
    Rad,
    /// Lifelink counter (grants lifelink, CR 122.1b).
    Lifelink,
    /// Level counters (classes, CR 716).
    Level,
    /// Card-specific counters.
    Custom(u16),
}

impl CounterKind {
    /// Parses a starting-position counter, including arbitrary signed P/T pairs.
    #[must_use]
    pub fn from_setup_name(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "loyalty" => Some(Self::Loyalty),
            "lore" => Some(Self::Lore),
            "time" => Some(Self::Time),
            "charge" => Some(Self::Charge),
            "poison" => Some(Self::Poison),
            "energy" => Some(Self::Energy),
            "rad" => Some(Self::Rad),
            "lifelink" => Some(Self::Lifelink),
            "level" => Some(Self::Level),
            value => {
                if let Some(id) = value.strip_prefix("custom:") {
                    return id.parse().ok().map(Self::Custom);
                }
                let (a, b) = value.split_once('/')?;
                if a.starts_with('+') && b.starts_with('+') {
                    Some(Self::Plus {
                        power: a[1..].parse().ok()?,
                        toughness: b[1..].parse().ok()?,
                    })
                } else if a.starts_with('-') && b.starts_with('-') {
                    Some(Self::Minus {
                        power: a[1..].parse().ok()?,
                        toughness: b[1..].parse().ok()?,
                    })
                } else {
                    None
                }
            }
        }
    }

    /// The +1/+1 counter, which Magic prints on more cards than every other
    /// P/T pair together (2528 reference scripts against 195).
    ///
    /// A constant and not a variant, so that there is exactly one value for
    /// it: `Plus { power: 1, toughness: 1 }` and this name are the same
    /// value and compare equal, where a variant beside the general form
    /// would be two spellings nothing could keep in step.
    pub const P1P1: Self = Self::Plus {
        power: 1,
        toughness: 1,
    };
    /// The −1/−1 counter — [`Self::P1P1`]'s partner in CR 704.5q.
    pub const M1M1: Self = Self::Minus {
        power: 1,
        toughness: 1,
    };

    /// What this counter adds to power and toughness (CR 122.1a), or `None`
    /// for a counter that changes neither.
    ///
    /// The one place the arithmetic is written. Layer 7c (CR 613.4c) sums it
    /// over every counter a permanent wears, which is what lets a creature
    /// carry a −0/−1 and a +1/+1 at once without either being special-cased.
    #[must_use]
    pub const fn power_toughness(self) -> Option<(i16, i16)> {
        match self {
            Self::Plus { power, toughness } => Some((power as i16, toughness as i16)),
            Self::Minus { power, toughness } => Some((-(power as i16), -(toughness as i16))),
            _ => None,
        }
    }
}

/// Definition of a token a card can create.
///
/// A token is a permanent with no card behind it, which for a long time also
/// meant it could carry no rules: the engine reads abilities off the card in
/// the registry, and a token has none. `abilities` closes that hole — it is
/// the same [`crate::AbilityDef`] slice a card face carries, so a Treasure's
/// "{T}, Sacrifice this artifact: Add one mana of any color" is written and
/// executed exactly like the identical ability printed on a real card.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TokenDef {
    /// Token name.
    pub name: &'static str,
    /// Colors.
    pub colors: ColorSet,
    /// Types.
    pub types: TypeSet,
    /// Supertypes.
    pub supertypes: SupertypeSet,
    /// Subtypes.
    pub subtypes: &'static [SubtypeId],
    /// Power (creatures).
    pub power: Option<i16>,
    /// Toughness (creatures).
    pub toughness: Option<i16>,
    /// Keywords.
    pub keywords: KeywordSet,
    /// Activated and triggered abilities, read exactly like a card face's.
    pub abilities: &'static [crate::ability::AbilityDef],
    /// A printed token card whose picture this token wears.
    ///
    /// The same third-party identifier [`crate::CardDef::scryfall_id`]
    /// carries, and here for the same reason: a token has no printing of its
    /// own in the game's print table, so without an id the client has nothing
    /// to draw and falls back to a flat coloured rectangle with the name
    /// written on it. Which is what a token looked like.
    ///
    /// It is rules data only in the sense that the rest of this struct is —
    /// the engine never reads it, and the picture is fetched at run time from
    /// a third party like every other card image (`docs/legal.md` §3). Empty
    /// means "no picture chosen", and a client draws the face instead rather
    /// than issuing a request that cannot succeed.
    pub scryfall_id: &'static str,
}

impl TokenDef {
    /// The blank token: no name, colorless, no types, no abilities.
    ///
    /// Every definition in `baylee_cards::tokens` is written as a
    /// struct-update tail on this, so adding a field does not mean editing
    /// every token — the same contract [`crate::CardDef::DEFAULT`] carries.
    pub const DEFAULT: Self = Self {
        name: "",
        colors: ColorSet::EMPTY,
        types: TypeSet::EMPTY,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        keywords: KeywordSet::EMPTY,
        abilities: &[],
        scryfall_id: "",
    };
}
