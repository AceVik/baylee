//! Hydroelectric Specimen // Hydroelectric Laboratory — {2}{U} — Creature — Weird // Land
//! Set: MH3 #240 — Modern Horizons 3 | Scryfall ID: 8689ecd7-e9a6-458b-99d2-6dbaca527f00 | Oracle ID: 573151f0-00d4-4a8a-8a09-745c5f376532
//! Face: Hydroelectric Specimen — {2}{U} — Creature — Weird
//! Face: Hydroelectric Laboratory —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 642,
    oracle_id: "573151f0-00d4-4a8a-8a09-745c5f376532",
    scryfall_id: "8689ecd7-e9a6-458b-99d2-6dbaca527f00",
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    faces: &[
    face! {
        name: "Hydroelectric Specimen",
        mana_cost: baylee_core::mana!("{2}{U}"),
        types: TypeSet::CREATURE,
        subtypes: &[subtypes::creature::WEIRD],
        power: Some(1),
        toughness: Some(4),
    },
    face! {
        name: "Hydroelectric Laboratory",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
