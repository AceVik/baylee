//! Escaped Shapeshifter — {3}{U}{U} — Creature — Shapeshifter
//! Oracle: As long as an opponent controls a creature with flying not named Escaped Shapeshifter, this creature has flying. The same is true for first strike, trample, and protection from any color.
//! Set: TMP #62 — Tempest | Scryfall ID: e0171d4f-c871-4a7f-821a-82b7f401e9ca | Oracle ID: 17f2342b-4dd8-445a-a7c1-4a15a5688294
// PARTIAL — Gains flying, first strike and trample from opposing creatures not named Escaped Shapeshifter; detecting protection from each color on opposing creatures is unsupported.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const NOT_SHAPESHIFTER: Filter = Filter::Not(&Filter::Named("Escaped Shapeshifter"));
const OPP_FLYING: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasKeyword(KeywordSet::FLYING),
    NOT_SHAPESHIFTER,
]);
const OPP_FIRST_STRIKE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasKeyword(KeywordSet::FIRST_STRIKE),
    NOT_SHAPESHIFTER,
]);
const OPP_TRAMPLE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasKeyword(KeywordSet::TRAMPLE),
    NOT_SHAPESHIFTER,
]);

card!(
    index = index::ESCAPED_SHAPESHIFTER,
    oracle_id = "17f2342b-4dd8-445a-a7c1-4a15a5688294",
    scryfall_id = "e0171d4f-c871-4a7f-821a-82b7f401e9ca",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "Gains flying, first strike and trample from opposing creatures not named Escaped Shapeshifter; detecting protection from each color on opposing creatures is unsupported."
    ),
    faces = &[face!(
        name = "Escaped Shapeshifter",
        mana_cost = mana!("{3}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SHAPESHIFTER],
        power = Some(3),
        toughness = Some(4),
    ),],
    // NOT SUPPORTED: conditional protection from each color.
    abilities = &[
        static_ability!(
            Filter::This,
            Modifier::AddKeyword(KeywordSet::FLYING),
            condition = Some(Condition::OpponentControlCount(&OPP_FLYING, 1)),
        ),
        static_ability!(
            Filter::This,
            Modifier::AddKeyword(KeywordSet::FIRST_STRIKE),
            condition = Some(Condition::OpponentControlCount(&OPP_FIRST_STRIKE, 1)),
        ),
        static_ability!(
            Filter::This,
            Modifier::AddKeyword(KeywordSet::TRAMPLE),
            condition = Some(Condition::OpponentControlCount(&OPP_TRAMPLE, 1)),
        ),
    ],
);
