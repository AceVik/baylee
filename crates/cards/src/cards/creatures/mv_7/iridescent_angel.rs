//! Iridescent Angel — {5}{W}{U} — Creature — Angel
//! Oracle: Flying, protection from each color
//! Set: ODY #288 — Odyssey | Scryfall ID: f3bce4f8-3a3f-4ec2-9b37-10bb12713afc | Oracle ID: 28fa9211-96bb-4fef-81bf-8f58114a8df7
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::IRIDESCENT_ANGEL,
    oracle_id = "28fa9211-96bb-4fef-81bf-8f58114a8df7",
    scryfall_id = "f3bce4f8-3a3f-4ec2-9b37-10bb12713afc",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    faces = &[face!(
        name = "Iridescent Angel",
        mana_cost = mana!("{5}{W}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ANGEL],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
