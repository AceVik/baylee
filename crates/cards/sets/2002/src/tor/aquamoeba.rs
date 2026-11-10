//! Aquamoeba — {1}{U} — Creature — Elemental Beast
//! Oracle: Discard a card: Switch this creature's power and toughness until end of turn.
//! Set: DMR #38 — Dominaria Remastered | Scryfall ID: 62721a55-37c6-4c05-aedf-0aa203045afa | Oracle ID: 55de434c-0fb9-4127-81b6-47c1c0b088fe
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::AQUAMOEBA,
    oracle_id = "55de434c-0fb9-4127-81b6-47c1c0b088fe",
    scryfall_id = "62721a55-37c6-4c05-aedf-0aa203045afa",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Aquamoeba",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELEMENTAL, subtypes::creature::BEAST],
        power = Some(1),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
