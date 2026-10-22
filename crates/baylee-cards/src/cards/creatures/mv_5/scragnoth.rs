//! Scragnoth — {4}{G} — Creature — Beast
//! Oracle: This spell can't be countered.
//! Oracle: Protection from blue
//! Set: TSB #83 — Time Spiral Timeshifted | Scryfall ID: 352c45b0-a20a-427c-8f2e-f461983ce968 | Oracle ID: 9e66f2d6-cc51-4747-914d-2c04e439da2b
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SCRAGNOTH,
    oracle_id = "9e66f2d6-cc51-4747-914d-2c04e439da2b",
    scryfall_id = "352c45b0-a20a-427c-8f2e-f461983ce968",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    keywords = KeywordSet::UNCOUNTERABLE,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Scragnoth",
        mana_cost = mana!("{4}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(3),
        toughness = Some(4),
    ),],
    abilities = &[static_ability!(
        Filter::This,
        Modifier::ProtectionFrom(&Filter::HasColor(ColorSet::from_slice(&[Color::Blue]))),
    )],
);
