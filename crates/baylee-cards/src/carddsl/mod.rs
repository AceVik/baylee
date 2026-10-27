//! Typed Card & Ability DSL encoder (`DSL_VERSION = 2`), shared by Expert 1
//! (Playernet v4) and the future Expert 2 (Deckbuilder / Synergy / Bracket
//! Evaluator).
//!
//! Unlike `features3::card_dsl`, which hashed `format!("{:?}", ...)` strings of
//! `Trigger`, `Effect`, `Cost`, `StaticAbility`, and `Filter` into 512 buckets
//! and applied `ln_1p` twice ( once in Rust and once in `dataset3.py`), this
//! module walks the typed AST in [`baylee_cards_dsl`] directly and produces:
//!
//! 1. A fixed-width card/face summary (`[f32; CARD_DSL_WIDTH]`, 160 floats)
//!    normalized once into `[-1.0, 1.0]` / `[0.0, 1.0]`, covering mana costs,
//!    types, supertypes, color identity, keywords, stats, ability counts,
//!    semantic effect channels, subtype coordinates, and deckbuilding synergy
//!    role tags.
//! 2. Per-ability structured rows (`[[f32; ABILITY_DSL_WIDTH]; MAX_ABILITIES]`,
//!    `8 x 64` floats) aligned with `CardDef::abilities_for_face(face)` (and
//!    `TokenDef::abilities`) so `ActivateAbility { ability_index }`,
//!    `StackItem::Ability { ability }`, and `GrantSource` can look up the exact
//!    structured representation of the referenced ability.

// The versioned model interface intentionally normalizes counts into f32.
#![allow(clippy::cast_precision_loss)]

use crate::dsl::{
    AbilityDef, ActivationLimit, ActivationTiming, ActivationZone, Amount, CardDef, CommanderRule,
    Cost, CostPart, DivisionAmount, Effect, EnterModifier, FaceDef, Filter, KeywordSet, Modifier,
    PartnerKind, ReplacementRule, StaticAbility, TargetReq, TargetSpec, TokenDef, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::{ManaCost, ManaSymbol};
use baylee_core::types::{SupertypeSet, TypeSet};

/// Schema version of the typed Card/Ability DSL encoder.
pub const DSL_VERSION: u32 = 2;

/// Missing card identity; registry index zero is a real card.
pub const UNKNOWN_CARD: u32 = u32::MAX;

/// Width of the card/face summary vector.
pub const CARD_DSL_WIDTH: usize = 160;

/// Maximum number of per-card/per-face abilities encoded individually.
pub const MAX_ABILITIES: usize = 8;

/// Width of each structured ability vector.
pub const ABILITY_DSL_WIDTH: usize = 64;

/// Complete typed DSL encoding of one card face or token definition.
#[derive(Clone, Debug, PartialEq)]
pub struct CardDslFeatures {
    /// Card/face summary vector (`160` floats, single-stage normalized).
    pub summary: [f32; CARD_DSL_WIDTH],
    /// Per-ability structured vectors (`8 x 64` floats).
    pub abilities: [[f32; ABILITY_DSL_WIDTH]; MAX_ABILITIES],
    /// Number of populated ability slots (`0..=MAX_ABILITIES`).
    pub ability_count: u8,
}

impl Default for CardDslFeatures {
    fn default() -> Self {
        Self {
            summary: [0.0; CARD_DSL_WIDTH],
            abilities: [[0.0; ABILITY_DSL_WIDTH]; MAX_ABILITIES],
            ability_count: 0,
        }
    }
}

/// Saturating linear scale into `[0.0, 1.0]`.
#[inline]
fn unit(v: f32, cap: f32) -> f32 {
    (v / cap).clamp(0.0, 1.0)
}

/// Signed linear scale into `[-1.0, 1.0]`.
#[inline]
fn signed_unit(v: f32, cap: f32) -> f32 {
    (v / cap).clamp(-1.0, 1.0)
}

/// Encodes a [`ManaCost`] into `10` floats:
/// `[mv, W, U, B, R, G, C, generic, X_count, hybrid_or_phyrexian]`.
#[must_use]
#[allow(
    clippy::many_single_char_names,
    reason = "Printed WUBRGC mana coordinates"
)]
pub fn encode_mana_cost(cost: &ManaCost) -> [f32; 10] {
    let mut w = 0.0_f32;
    let mut u = 0.0_f32;
    let mut b = 0.0_f32;
    let mut r = 0.0_f32;
    let mut g = 0.0_f32;
    let mut c = 0.0_f32;
    let mut generic = 0.0_f32;
    let mut x = 0.0_f32;
    let mut special = 0.0_f32;
    for sym in cost.symbols() {
        match sym {
            ManaSymbol::White => w += 1.0,
            ManaSymbol::Blue => u += 1.0,
            ManaSymbol::Black => b += 1.0,
            ManaSymbol::Red => r += 1.0,
            ManaSymbol::Green => g += 1.0,
            ManaSymbol::Colorless => c += 1.0,
            ManaSymbol::Generic(n) => generic += n as f32,
            ManaSymbol::Variable(_) => x += 1.0,
            ManaSymbol::Hybrid(_)
            | ManaSymbol::TwoOrColor(_)
            | ManaSymbol::Phyrexian(_)
            | ManaSymbol::HybridPhyrexian(_)
            | ManaSymbol::Snow
            | ManaSymbol::HalfGeneric
            | ManaSymbol::Infinite => {
                special += 1.0;
                let cols = sym.colors();
                for (col, flag) in [
                    (Color::White, &mut w),
                    (Color::Blue, &mut u),
                    (Color::Black, &mut b),
                    (Color::Red, &mut r),
                    (Color::Green, &mut g),
                ] {
                    if cols.contains(col) {
                        *flag += 0.5;
                    }
                }
            }
        }
    }
    [
        unit(cost.cmc() as f32, 12.0),
        unit(w, 6.0),
        unit(u, 6.0),
        unit(b, 6.0),
        unit(r, 6.0),
        unit(g, 6.0),
        unit(c, 4.0),
        unit(generic, 10.0),
        unit(x, 3.0),
        unit(special, 4.0),
    ]
}

fn encode_color_set(colors: ColorSet) -> [f32; 5] {
    [
        f32::from(u8::from(colors.contains(Color::White))),
        f32::from(u8::from(colors.contains(Color::Blue))),
        f32::from(u8::from(colors.contains(Color::Black))),
        f32::from(u8::from(colors.contains(Color::Red))),
        f32::from(u8::from(colors.contains(Color::Green))),
    ]
}

fn encode_type_set(types: TypeSet) -> [f32; 9] {
    [
        f32::from(u8::from(types.contains(TypeSet::CREATURE))),
        f32::from(u8::from(types.contains(TypeSet::INSTANT))),
        f32::from(u8::from(types.contains(TypeSet::SORCERY))),
        f32::from(u8::from(types.contains(TypeSet::ARTIFACT))),
        f32::from(u8::from(types.contains(TypeSet::ENCHANTMENT))),
        f32::from(u8::from(types.contains(TypeSet::LAND))),
        f32::from(u8::from(types.contains(TypeSet::PLANESWALKER))),
        f32::from(u8::from(types.contains(TypeSet::BATTLE))),
        f32::from(u8::from(types.contains(TypeSet::KINDRED))),
    ]
}

fn encode_supertype_set(supertypes: SupertypeSet) -> [f32; 4] {
    [
        f32::from(u8::from(supertypes.contains(SupertypeSet::BASIC))),
        f32::from(u8::from(supertypes.contains(SupertypeSet::LEGENDARY))),
        f32::from(u8::from(supertypes.contains(SupertypeSet::SNOW))),
        f32::from(u8::from(supertypes.contains(SupertypeSet::WORLD))),
    ]
}

/// Core 24 keyword bits used across both summary and modifier projections.
const CORE_KEYWORDS: [KeywordSet; 24] = [
    KeywordSet::FLYING,
    KeywordSet::FIRST_STRIKE,
    KeywordSet::DOUBLE_STRIKE,
    KeywordSet::DEATHTOUCH,
    KeywordSet::HASTE,
    KeywordSet::HEXPROOF,
    KeywordSet::INDESTRUCTIBLE,
    KeywordSet::LIFELINK,
    KeywordSet::MENACE,
    KeywordSet::REACH,
    KeywordSet::TRAMPLE,
    KeywordSet::VIGILANCE,
    KeywordSet::DEFENDER,
    KeywordSet::FLASH,
    KeywordSet::SHROUD,
    KeywordSet::INFECT,
    KeywordSet::WITHER,
    KeywordSet::PERSIST,
    KeywordSet::UNDYING,
    KeywordSet::PROWESS,
    KeywordSet::CHANGELING,
    KeywordSet::UNBLOCKABLE,
    KeywordSet::UNCOUNTERABLE,
    KeywordSet::STORM,
];

include!("effects.rs");
include!("abilities.rs");
include!("cards.rs");
include!("tests.rs");
