//! Central token definitions — one constant per named token, referenced
//! by card files instead of duplicating `TokenDef` literals. Each token
//! has a stable `token_id` (its index in [`ALL`]) so clients can map it
//! to token art.
//!
//! Add new tokens here (keep `ALL` in sync — the id IS the art key).
//!
//! # Why a token may not be defined in a card file
//!
//! The index into [`ALL`] is what the engine stamps on the object it creates
//! and what the view hands the client, so a `TokenDef` that lives anywhere
//! else has no id: [`token_id`] answers `u16::MAX` for it, and the token
//! reaches the table nameless as far as art is concerned. Two cards did
//! exactly that and quietly lost their tokens' identity. The test at the foot
//! of this file is what turns the convention into a build failure.
//!
//! # Abilities
//!
//! A token carries its own abilities ([`TokenDef::abilities`]), read by the
//! engine through the same path a card face's are. That is what makes a
//! Treasure a Treasure rather than a blank artifact: before it existed, every
//! token on the battlefield was inert whatever its name said.

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, Amount, Cost, CostPart, Effect, Filter,
    KeywordSet, TokenDef,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{artifact, creature};
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::TypeSet;

/// The five colors, for "any color" mana abilities.
static ANY_COLOR: &[ManaColor] = &[
    ManaColor::White,
    ManaColor::Blue,
    ManaColor::Black,
    ManaColor::Red,
    ManaColor::Green,
];

/// `{T}, Sacrifice this artifact: Add one mana of any color.` (Treasure)
static SACRIFICE_FOR_ANY_COLOR: &[AbilityDef] = &[AbilityDef::Activated {
    cost: Cost {
        mana: ManaCost::ZERO,
        parts: &[CostPart::TapSelf, CostPart::SacrificeSelf],
    },
    effects: &[Effect::AddManaChoice {
        colors: ANY_COLOR,
        amount: Amount::Fixed(1),
        combination: false,
    }],
    target: None,
    timing: ActivationTiming::InstantSpeed,
    // A mana ability (CR 605.1a): no target, adds mana, is not itself an
    // activated ability that uses the stack — which is what lets a Treasure
    // be cracked while paying for a spell.
    mana_ability: true,
    zone: ActivationZone::Battlefield,
}];

/// `{2}, Sacrifice this artifact: Draw a card.` (Clue)
static SACRIFICE_TO_DRAW: &[AbilityDef] = &[AbilityDef::Activated {
    cost: Cost {
        mana: ManaCost::parse("{2}"),
        parts: &[CostPart::SacrificeSelf],
    },
    effects: &[Effect::DrawCards {
        amount: Amount::Fixed(1),
    }],
    target: None,
    timing: ActivationTiming::InstantSpeed,
    mana_ability: false,
    zone: ActivationZone::Battlefield,
}];

/// `{2}, {T}, Sacrifice this artifact: You gain 3 life.` (Food)
static SACRIFICE_TO_GAIN_LIFE: &[AbilityDef] = &[AbilityDef::Activated {
    cost: Cost {
        mana: ManaCost::parse("{2}"),
        parts: &[CostPart::TapSelf, CostPart::SacrificeSelf],
    },
    effects: &[Effect::GainLife {
        amount: Amount::Fixed(3),
    }],
    target: None,
    timing: ActivationTiming::InstantSpeed,
    mana_ability: false,
    zone: ActivationZone::Battlefield,
}];

/// `{1}, {T}, Discard a card, Sacrifice this artifact: Draw a card.` (Blood)
static SACRIFICE_TO_LOOT: &[AbilityDef] = &[AbilityDef::Activated {
    cost: Cost {
        mana: ManaCost::parse("{1}"),
        parts: &[
            CostPart::TapSelf,
            CostPart::Discard(&Filter::Any),
            CostPart::SacrificeSelf,
        ],
    },
    effects: &[Effect::DrawCards {
        amount: Amount::Fixed(1),
    }],
    target: None,
    timing: ActivationTiming::InstantSpeed,
    mana_ability: false,
    zone: ActivationZone::Battlefield,
}];

/// 1/1 white Ally (Aang and Katara, Jasmine Dragon Tea Shop, Sokka).
pub static ALLY_1_1_WHITE: TokenDef = TokenDef {
    name: "Ally",
    colors: ColorSet::from_slice(&[Color::White]),
    types: TypeSet::CREATURE,
    subtypes: &[creature::ALLY],
    power: Some(1),
    toughness: Some(1),
    ..TokenDef::DEFAULT
};

/// 4/4 white Angel with flying (Luminarch Ascension).
pub static ANGEL_4_4_WHITE_FLYING: TokenDef = TokenDef {
    name: "Angel",
    colors: ColorSet::from_slice(&[Color::White]),
    types: TypeSet::CREATURE,
    subtypes: &[creature::ANGEL],
    power: Some(4),
    toughness: Some(4),
    keywords: KeywordSet::FLYING,
    ..TokenDef::DEFAULT
};

/// 0/0 black Army (amass, CR 701.44a — the token the mechanic creates when
/// you control no Army; the counters go on afterwards, and "amass Orcs"
/// adds the Orc type on top of this).
pub static ARMY_0_0_BLACK: TokenDef = TokenDef {
    name: "Army",
    colors: ColorSet::from_slice(&[Color::Black]),
    types: TypeSet::CREATURE,
    subtypes: &[creature::ARMY],
    power: Some(0),
    toughness: Some(0),
    ..TokenDef::DEFAULT
};

/// 1/1 white Bird with flying (Aether Channeler).
pub static BIRD_1_1_WHITE_FLYING: TokenDef = TokenDef {
    name: "Bird",
    colors: ColorSet::from_slice(&[Color::White]),
    types: TypeSet::CREATURE,
    subtypes: &[creature::BIRD],
    power: Some(1),
    toughness: Some(1),
    keywords: KeywordSet::FLYING,
    ..TokenDef::DEFAULT
};

/// Colorless Blood artifact: `{1}, {T}, Discard a card, Sacrifice this
/// artifact: Draw a card.`
pub static BLOOD: TokenDef = TokenDef {
    name: "Blood",
    types: TypeSet::ARTIFACT,
    subtypes: &[artifact::BLOOD],
    abilities: SACRIFICE_TO_LOOT,
    ..TokenDef::DEFAULT
};

/// 2/2 green Boar (Curse of the Swine).
pub static BOAR_2_2_GREEN: TokenDef = TokenDef {
    name: "Boar",
    colors: ColorSet::from_slice(&[Color::Green]),
    types: TypeSet::CREATURE,
    subtypes: &[creature::BOAR],
    power: Some(2),
    toughness: Some(2),
    ..TokenDef::DEFAULT
};

/// Colorless Clue artifact: `{2}, Sacrifice this artifact: Draw a card.`
pub static CLUE: TokenDef = TokenDef {
    name: "Clue",
    types: TypeSet::ARTIFACT,
    subtypes: &[artifact::CLUE],
    abilities: SACRIFICE_TO_DRAW,
    ..TokenDef::DEFAULT
};

/// 0/0 colorless Construct artifact creature (Urza's Saga). Its size comes
/// from a continuous effect the card registers, not from the token.
pub static CONSTRUCT_0_0: TokenDef = TokenDef {
    name: "Construct",
    types: TypeSet::CREATURE.union(TypeSet::ARTIFACT),
    subtypes: &[creature::CONSTRUCT],
    power: Some(0),
    toughness: Some(0),
    ..TokenDef::DEFAULT
};

/// Colorless Food artifact: `{2}, {T}, Sacrifice this artifact: You gain
/// 3 life.`
pub static FOOD: TokenDef = TokenDef {
    name: "Food",
    types: TypeSet::ARTIFACT,
    subtypes: &[artifact::FOOD],
    abilities: SACRIFICE_TO_GAIN_LIFE,
    ..TokenDef::DEFAULT
};

/// Blue Illusion with no printed size (Skyclave Apparition): the card sets
/// its power and toughness to the exiled card's mana value as it is created,
/// so leaving them unset here is what lets that effect speak.
pub static ILLUSION_X_BLUE: TokenDef = TokenDef {
    name: "Illusion",
    colors: ColorSet::from_slice(&[Color::Blue]),
    types: TypeSet::CREATURE,
    subtypes: &[creature::ILLUSION],
    ..TokenDef::DEFAULT
};

/// 1/1 colorless Shapeshifter with changeling (Crib Swap).
pub static SHAPESHIFTER_1_1_CHANGELING: TokenDef = TokenDef {
    name: "Shapeshifter",
    types: TypeSet::CREATURE,
    subtypes: &[creature::SHAPESHIFTER],
    power: Some(1),
    toughness: Some(1),
    keywords: KeywordSet::CHANGELING,
    ..TokenDef::DEFAULT
};

/// 2/2 blue Shapeshifter with changeling (Maskwood Nexus).
pub static SHAPESHIFTER_2_2_BLUE_CHANGELING: TokenDef = TokenDef {
    name: "Shapeshifter",
    colors: ColorSet::from_slice(&[Color::Blue]),
    types: TypeSet::CREATURE,
    subtypes: &[creature::SHAPESHIFTER],
    power: Some(2),
    toughness: Some(2),
    keywords: KeywordSet::CHANGELING,
    ..TokenDef::DEFAULT
};

/// 1/1 white Soldier (Elspeth, Storm Slayer).
pub static SOLDIER_1_1_WHITE: TokenDef = TokenDef {
    name: "Soldier",
    colors: ColorSet::from_slice(&[Color::White]),
    types: TypeSet::CREATURE,
    subtypes: &[creature::SOLDIER],
    power: Some(1),
    toughness: Some(1),
    ..TokenDef::DEFAULT
};

/// Colorless Treasure artifact: `{T}, Sacrifice this artifact: Add one mana
/// of any color.` (Smothering Tithe)
pub static TREASURE: TokenDef = TokenDef {
    name: "Treasure",
    types: TypeSet::ARTIFACT,
    subtypes: &[artifact::TREASURE],
    abilities: SACRIFICE_FOR_ANY_COLOR,
    ..TokenDef::DEFAULT
};

/// All central tokens; the index IS the stable token id (art key).
///
/// Append only — an insertion in the middle renumbers every token after it,
/// and the number is what a client has cached as an art key.
pub static ALL: &[&TokenDef] = &[
    &ALLY_1_1_WHITE,
    &ANGEL_4_4_WHITE_FLYING,
    &BIRD_1_1_WHITE_FLYING,
    &BOAR_2_2_GREEN,
    &CONSTRUCT_0_0,
    &ILLUSION_X_BLUE,
    &SHAPESHIFTER_1_1_CHANGELING,
    &SHAPESHIFTER_2_2_BLUE_CHANGELING,
    &SOLDIER_1_1_WHITE,
    &TREASURE,
    &ARMY_0_0_BLACK,
    &BLOOD,
    &CLUE,
    &FOOD,
];

/// The stable id of a central token (its index in [`ALL`]).
#[must_use]
pub fn token_id(token: &TokenDef) -> u16 {
    ALL.iter()
        .position(|t| std::ptr::eq(*t, token))
        .map_or(u16::MAX, |i| i as u16)
}

/// The token a stable id names, if it names one.
#[must_use]
pub fn by_token_id(id: u16) -> Option<&'static TokenDef> {
    ALL.get(id as usize).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The id is the art key, so it has to survive the round trip. A token
    /// left out of [`ALL`] answers `u16::MAX` and reaches the client with no
    /// identity at all — the failure this test exists to catch.
    #[test]
    fn every_token_answers_to_the_id_it_is_filed_under() {
        for (i, token) in ALL.iter().enumerate() {
            let id = token_id(token);
            assert_eq!(id as usize, i, "{} is misfiled", token.name);
            assert!(std::ptr::eq(by_token_id(id).expect("round trip"), *token));
        }
        assert!(by_token_id(u16::MAX).is_none());
    }

    /// Every card that creates a token must name one from this file. A
    /// `TokenDef` literal in a card file compiles and works, but it has no
    /// id, so the token loses its art the moment it reaches the table — the
    /// bug Urza's Saga and Skyclave Apparition both carried.
    #[test]
    fn no_card_file_defines_its_own_token() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src/cards");
        let mut offenders = Vec::new();
        for entry in std::fs::read_dir(dir).expect("cards dir") {
            let path = entry.expect("dir entry").path();
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("read card");
            // The import line names the type without constructing one; a
            // literal is the `TokenDef {` that follows a `static` or `let`.
            if text.contains("TokenDef {") {
                offenders.push(
                    path.file_name()
                        .expect("file name")
                        .to_string_lossy()
                        .into_owned(),
                );
            }
        }
        offenders.sort();
        assert!(
            offenders.is_empty(),
            "these card files define tokens instead of using `crate::tokens`: {offenders:?}"
        );
    }

    /// A token whose name promises a sacrifice outlet and delivers nothing is
    /// worse than no token: the player sees a Treasure and cannot spend it.
    #[test]
    fn the_artifact_tokens_all_carry_their_printed_ability() {
        for token in [&TREASURE, &CLUE, &FOOD, &BLOOD] {
            assert!(
                !token.abilities.is_empty(),
                "{} has no ability to activate",
                token.name
            );
        }
    }
}
