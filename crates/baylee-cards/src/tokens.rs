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
    AbilityDef, Effect, Filter, KeywordSet, TokenDef, activated, cost, mana_ability,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{artifact, creature};
use baylee_core::mana::ManaColor;
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
// A mana ability (CR 605.1a): no target, adds mana, is not itself an
// activated ability that uses the stack — which is what lets a Treasure be
// cracked while paying for a spell. `mana_ability!` is that statement; the
// flag it sets is the one a card must never write by hand.
static SACRIFICE_FOR_ANY_COLOR: &[AbilityDef] = &[mana_ability!(
    cost!(TapSelf, SacrificeSelf),
    &[Effect::mana_choice(ANY_COLOR)]
)];

/// `{2}, Sacrifice this artifact: Draw a card.` (Clue)
static SACRIFICE_TO_DRAW: &[AbilityDef] =
    &[activated!(cost!("{2}", SacrificeSelf), &[Effect::draw(1)])];

/// `{2}, {T}, Sacrifice this artifact: You gain 3 life.` (Food)
static SACRIFICE_TO_GAIN_LIFE: &[AbilityDef] = &[activated!(
    cost!("{2}", TapSelf, SacrificeSelf),
    &[Effect::gain_life(3)]
)];

/// `{1}, {T}, Discard a card, Sacrifice this artifact: Draw a card.` (Blood)
static SACRIFICE_TO_LOOT: &[AbilityDef] = &[activated!(
    cost!("{1}", TapSelf, Discard(&Filter::Any), SacrificeSelf),
    &[Effect::draw(1)]
)];

/// 1/1 white Ally (Aang and Katara, Jasmine Dragon Tea Shop, Sokka).
pub static ALLY_1_1_WHITE: TokenDef = TokenDef {
    name: "Ally",
    colors: ColorSet::from_slice(&[Color::White]),
    types: TypeSet::CREATURE,
    subtypes: &[creature::ALLY],
    power: Some(1),
    toughness: Some(1),
    // TLA #8 — Avatar: The Last Airbender, the set the three cards that
    // make this token come from.
    scryfall_id: "01439983-8394-4a0c-9e9e-92a3f1927fe3",
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
    // SOI #1 — flying and nothing else. Most Angel tokens in print are
    // 4/4 flying *and vigilance*, and their picture says so on the card.
    scryfall_id: "bdb975fe-ac30-4249-bb55-7efb64645e4d",
    ..TokenDef::DEFAULT
};

/// 0/0 black Army (amass, CR 701.47a — the token the mechanic creates when
/// you control no Army; the counters go on afterwards, and "amass Orcs"
/// adds the Orc type on top of this).
pub static ARMY_0_0_BLACK: TokenDef = TokenDef {
    name: "Army",
    colors: ColorSet::from_slice(&[Color::Black]),
    types: TypeSet::CREATURE,
    subtypes: &[creature::ARMY],
    power: Some(0),
    toughness: Some(0),
    // LTR #6, an Orc Army: no card prints a bare "Army", because amass
    // always names a creature type, and Orcish Bowmasters is the only
    // card in the pool that makes one.
    scryfall_id: "db598f33-2ff9-4e0b-a067-05fecc03435f",
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
    // DMU #2, Aether Channeler's own set.
    scryfall_id: "5f3034f6-145f-4e60-9e55-c4054fd8e70f",
    ..TokenDef::DEFAULT
};

/// Colorless Blood artifact: `{1}, {T}, Discard a card, Sacrifice this
/// artifact: Draw a card.`
pub static BLOOD: TokenDef = TokenDef {
    name: "Blood",
    types: TypeSet::ARTIFACT,
    subtypes: &[artifact::BLOOD],
    abilities: SACRIFICE_TO_LOOT,
    // VOW #17, the set that introduced Blood.
    scryfall_id: "a6f374bc-cd29-469f-808a-6a6c004ee8aa",
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
    // THS #8, Curse of the Swine's own set.
    scryfall_id: "2f40613b-1bde-4939-86ad-6bd40f9db0d6",
    ..TokenDef::DEFAULT
};

/// Colorless Clue artifact: `{2}, Sacrifice this artifact: Draw a card.`
pub static CLUE: TokenDef = TokenDef {
    name: "Clue",
    types: TypeSet::ARTIFACT,
    subtypes: &[artifact::CLUE],
    abilities: SACRIFICE_TO_DRAW,
    // SOI #13, the set that introduced Clues.
    scryfall_id: "271afa7e-2126-4497-b871-9795b7355d69",
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
    // MH2 #16, Urza's Saga's own set.
    scryfall_id: "a7caaf39-8f16-4f1d-bee6-a45674306319",
    ..TokenDef::DEFAULT
};

/// Colorless Food artifact: `{2}, {T}, Sacrifice this artifact: You gain
/// 3 life.`
pub static FOOD: TokenDef = TokenDef {
    name: "Food",
    types: TypeSet::ARTIFACT,
    subtypes: &[artifact::FOOD],
    abilities: SACRIFICE_TO_GAIN_LIFE,
    // ELD #15, the set that introduced Food.
    scryfall_id: "bf36408d-ed85-497f-8e68-d3a922c388a0",
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
    // ZNR #6, Skyclave Apparition's own set — and the one Illusion token
    // in print with no fixed size, which is what this token is.
    scryfall_id: "2300635e-7771-4676-a5a5-29a9d8f49f1a",
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
    // LRW #11, Crib Swap's own set.
    scryfall_id: "1a7d89ca-8611-4bda-b5c8-0350ce091102",
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
    // KHM #8, Maskwood Nexus's own set.
    scryfall_id: "ef775ad0-b1a9-4254-ab6f-304558bb77a1",
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
    // DMU #4. Elspeth's own set prints no Soldier token of its own, and a
    // 1/1 white Soldier is the same card in every set that does.
    scryfall_id: "8c4b0257-2ca5-4015-9d63-d7cf6e87ab9d",
    ..TokenDef::DEFAULT
};

/// Colorless Treasure artifact: `{T}, Sacrifice this artifact: Add one mana
/// of any color.` (Smothering Tithe)
pub static TREASURE: TokenDef = TokenDef {
    name: "Treasure",
    types: TypeSet::ARTIFACT,
    subtypes: &[artifact::TREASURE],
    abilities: SACRIFICE_FOR_ANY_COLOR,
    // RNA #12, Smothering Tithe's own set.
    scryfall_id: "0559f9f3-eff0-465d-93c1-e875a8afe87f",
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
    ///
    /// Through [`crate::tests::every_card_file`], because this read `src/cards`
    /// with a flat `read_dir` for as long as the taxonomy has existed — one
    /// `mod.rs` and no cards, an empty worklist reported as a clean pool.
    #[test]
    fn no_card_file_defines_its_own_token() {
        let mut offenders = Vec::new();
        for (name, text) in crate::tests::every_card_file() {
            // The import line names the type without constructing one; a
            // literal is the `TokenDef {` that follows a `static` or `let`.
            if text.contains("TokenDef {") {
                offenders.push(name);
            }
        }
        offenders.sort();
        assert!(
            offenders.is_empty(),
            "these card files define tokens instead of using `crate::tokens`: {offenders:?}"
        );
    }

    /// A token with no picture is the flat coloured rectangle this file's
    /// art keys exist to replace, and a *malformed* id is worse than none:
    /// the client builds a URL out of it and fetches a guaranteed 404 every
    /// time the token is drawn. Both are build failures, because neither is
    /// visible from anywhere but the felt.
    ///
    /// The shape checked here is the one `baylee_client_core::images` will
    /// accept — a hyphenated 36-character UUID that is not the nil one.
    #[test]
    fn every_token_names_a_picture_the_client_can_fetch() {
        for token in ALL {
            let id = token.scryfall_id;
            assert!(!id.is_empty(), "{} has no art", token.name);
            assert_eq!(id.len(), 36, "{}: {id} is not a UUID", token.name);
            assert!(id.contains('-'), "{}: {id} is not a UUID", token.name);
            assert!(
                !id.chars().all(|c| c == '0' || c == '-'),
                "{}: the nil UUID is well-formed and always wrong",
                token.name
            );
            assert!(
                id.chars().all(|c| c.is_ascii_hexdigit() || c == '-'),
                "{}: {id} is not hexadecimal",
                token.name
            );
        }
    }

    /// Two tokens sharing a picture is not an error — a 1/1 and a 2/2
    /// Shapeshifter could reasonably wear the same art — but it has never
    /// been what was *meant* here, and a copied line is how it would happen.
    #[test]
    fn no_two_tokens_were_given_the_same_picture_by_accident() {
        let mut ids: Vec<&str> = ALL.iter().map(|t| t.scryfall_id).collect();
        ids.sort_unstable();
        let before = ids.len();
        ids.dedup();
        assert_eq!(before, ids.len(), "two tokens share an art id");
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
