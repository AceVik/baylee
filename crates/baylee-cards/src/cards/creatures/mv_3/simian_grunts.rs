//! Simian Grunts — {2}{G} — Creature — Ape
//! Oracle: Flash
//! Oracle: Echo {2}{G} (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)
//! Set: VMA #232 — Vintage Masters | Scryfall ID: 15104b03-b122-40ba-ad9f-51289f627a19 | Oracle ID: 158f4e87-60f1-43c1-b03d-9bbe4b824b39
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SIMIAN_GRUNTS,
    oracle_id = "158f4e87-60f1-43c1-b03d-9bbe4b824b39",
    scryfall_id = "15104b03-b122-40ba-ad9f-51289f627a19",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    keywords = KeywordSet::FLASH,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Simian Grunts",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::APE],
        power = Some(3),
        toughness = Some(4),
    ),],
    abilities = &[AbilityDef::Echo {
        cost: mana!("{2}{G}"),
    }],
);
