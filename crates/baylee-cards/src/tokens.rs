//! Central token definitions — one constant per named token, referenced
//! by card files instead of duplicating `TokenDef` literals. Each token
//! has a stable `token_id` (its index in [`ALL`]) so clients can map it
//! to token art.
//!
//! Add new tokens here (keep `ALL` in sync — the id IS the art key).

use baylee_cards_dsl::{KeywordSet, TokenDef};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{artifact, creature};
use baylee_core::types::{SupertypeSet, TypeSet};

/// 1/1 white Ally (Aang and Katara, Jasmine Dragon Tea Shop, Sokka).
pub static ALLY_1_1_WHITE: TokenDef = TokenDef {
    name: "Ally",
    colors: ColorSet::from_slice(&[Color::White]),
    types: TypeSet::CREATURE,
    supertypes: SupertypeSet::EMPTY,
    subtypes: &[creature::ALLY],
    power: Some(1),
    toughness: Some(1),
    keywords: KeywordSet::EMPTY,
};

/// 4/4 white Angel with flying (Luminarch Ascension).
pub static ANGEL_4_4_WHITE_FLYING: TokenDef = TokenDef {
    name: "Angel",
    colors: ColorSet::from_slice(&[Color::White]),
    types: TypeSet::CREATURE,
    supertypes: SupertypeSet::EMPTY,
    subtypes: &[creature::ANGEL],
    power: Some(4),
    toughness: Some(4),
    keywords: KeywordSet::FLYING,
};

/// 1/1 white Bird with flying (Aether Channeler).
pub static BIRD_1_1_WHITE_FLYING: TokenDef = TokenDef {
    name: "Bird",
    colors: ColorSet::from_slice(&[Color::White]),
    types: TypeSet::CREATURE,
    supertypes: SupertypeSet::EMPTY,
    subtypes: &[creature::BIRD],
    power: Some(1),
    toughness: Some(1),
    keywords: KeywordSet::FLYING,
};

/// 2/2 green Boar (Curse of the Swine).
pub static BOAR_2_2_GREEN: TokenDef = TokenDef {
    name: "Boar",
    colors: ColorSet::from_slice(&[Color::Green]),
    types: TypeSet::CREATURE,
    supertypes: SupertypeSet::EMPTY,
    subtypes: &[creature::BOAR],
    power: Some(2),
    toughness: Some(2),
    keywords: KeywordSet::EMPTY,
};

/// 0/0 colorless Construct artifact creature (Urza's Saga).
pub static CONSTRUCT_0_0: TokenDef = TokenDef {
    name: "Construct",
    colors: ColorSet::EMPTY,
    types: TypeSet::CREATURE.union(TypeSet::ARTIFACT),
    supertypes: SupertypeSet::EMPTY,
    subtypes: &[creature::CONSTRUCT],
    power: Some(0),
    toughness: Some(0),
    keywords: KeywordSet::EMPTY,
};

/// 1/1 blue Illusion (Skyclave Apparition).
pub static ILLUSION_1_1_BLUE: TokenDef = TokenDef {
    name: "Illusion",
    colors: ColorSet::from_slice(&[Color::Blue]),
    types: TypeSet::CREATURE,
    supertypes: SupertypeSet::EMPTY,
    subtypes: &[creature::ILLUSION],
    power: Some(1),
    toughness: Some(1),
    keywords: KeywordSet::EMPTY,
};

/// 1/1 colorless Shapeshifter with changeling (Crib Swap).
pub static SHAPESHIFTER_1_1_CHANGELING: TokenDef = TokenDef {
    name: "Shapeshifter",
    colors: ColorSet::EMPTY,
    types: TypeSet::CREATURE,
    supertypes: SupertypeSet::EMPTY,
    subtypes: &[creature::SHAPESHIFTER],
    power: Some(1),
    toughness: Some(1),
    keywords: KeywordSet::CHANGELING,
};

/// 2/2 blue Shapeshifter with changeling (Maskwood Nexus).
pub static SHAPESHIFTER_2_2_BLUE_CHANGELING: TokenDef = TokenDef {
    name: "Shapeshifter",
    colors: ColorSet::from_slice(&[Color::Blue]),
    types: TypeSet::CREATURE,
    supertypes: SupertypeSet::EMPTY,
    subtypes: &[creature::SHAPESHIFTER],
    power: Some(2),
    toughness: Some(2),
    keywords: KeywordSet::CHANGELING,
};

/// 1/1 white Soldier (Elspeth, Storm Slayer).
pub static SOLDIER_1_1_WHITE: TokenDef = TokenDef {
    name: "Soldier",
    colors: ColorSet::from_slice(&[Color::White]),
    types: TypeSet::CREATURE,
    supertypes: SupertypeSet::EMPTY,
    subtypes: &[creature::SOLDIER],
    power: Some(1),
    toughness: Some(1),
    keywords: KeywordSet::EMPTY,
};

/// Colorless Treasure artifact (Smothering Tithe).
pub static TREASURE: TokenDef = TokenDef {
    name: "Treasure",
    colors: ColorSet::EMPTY,
    types: TypeSet::ARTIFACT,
    supertypes: SupertypeSet::EMPTY,
    subtypes: &[artifact::TREASURE],
    power: None,
    toughness: None,
    keywords: KeywordSet::EMPTY,
};

/// All central tokens; the index IS the stable token id (art key).
pub static ALL: &[&TokenDef] = &[
    &ALLY_1_1_WHITE,
    &ANGEL_4_4_WHITE_FLYING,
    &BIRD_1_1_WHITE_FLYING,
    &BOAR_2_2_GREEN,
    &CONSTRUCT_0_0,
    &ILLUSION_1_1_BLUE,
    &SHAPESHIFTER_1_1_CHANGELING,
    &SHAPESHIFTER_2_2_BLUE_CHANGELING,
    &SOLDIER_1_1_WHITE,
    &TREASURE,
];

/// The stable id of a central token (its index in [`ALL`]).
#[must_use]
pub fn token_id(token: &TokenDef) -> u16 {
    ALL.iter()
        .position(|t| std::ptr::eq(*t, token))
        .map_or(u16::MAX, |i| i as u16)
}
