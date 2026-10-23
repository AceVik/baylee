//! Soldevi Adnate — {1}{B} — Creature — Human Cleric
//! Oracle: {T}, Sacrifice a black or artifact creature: Add an amount of {B} equal to the sacrificed creature's mana value.
//! Set: ALL #60a — Alliances | Scryfall ID: 8b2651b0-1ab2-4d7e-834f-7505797da474 | Oracle ID: e3174070-542f-429e-9149-b572f6614537
// IMPLEMENTED — {T}, sacrifice a black or artifact creature: add {B} equal to its mana value.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static BLACK_OR_ARTIFACT_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Or(&[
        Filter::HasColor(ColorSet::from_slice(&[Color::Black])),
        Filter::ARTIFACT,
    ]),
]);

card!(
    index = index::SOLDEVI_ADNATE,
    oracle_id = "e3174070-542f-429e-9149-b572f6614537",
    scryfall_id = "8b2651b0-1ab2-4d7e-834f-7505797da474",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Soldevi Adnate",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(2),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[mana_ability!(
        cost!(TapSelf, Sacrifice(&BLACK_OR_ARTIFACT_CREATURE)),
        &[Effect::mana_dynamic(
            ManaColor::Black,
            Amount::SacrificedManaValue
        )]
    )],
);
