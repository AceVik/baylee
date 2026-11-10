//! Priest of Yawgmoth — {1}{B} — Creature — Phyrexian Human Cleric
//! Oracle: {T}, Sacrifice an artifact: Add an amount of {B} equal to the sacrificed artifact's mana value.
//! Set: ATQ #19 — Antiquities | Scryfall ID: c9fd4054-42fc-4f95-a6f7-369a5da43dd5 | Oracle ID: cb6465f9-dcf8-4258-aa18-661ad252b58b

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PRIEST_OF_YAWGMOTH,
    oracle_id = "cb6465f9-dcf8-4258-aa18-661ad252b58b",
    scryfall_id = "c9fd4054-42fc-4f95-a6f7-369a5da43dd5",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Priest of Yawgmoth",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::PHYREXIAN,
            subtypes::creature::HUMAN,
            subtypes::creature::CLERIC
        ],
        power = Some(1),
        toughness = Some(2),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[mana_ability!(
        cost!(TapSelf, Sacrifice(&Filter::YOUR_ARTIFACT)),
        &[Effect::mana_dynamic(
            ManaColor::Black,
            Amount::SacrificedManaValue
        )]
    ),],
);
