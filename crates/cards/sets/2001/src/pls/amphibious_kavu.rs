//! Amphibious Kavu — {2}{G} — Creature — Kavu
//! Oracle: Whenever this creature blocks or becomes blocked by one or more blue and/or black creatures, this creature gets +3/+3 until end of turn.
//! Set: PLS #78 — Planeshift | Scryfall ID: 37d94fb2-958c-487e-9f64-52d2771c6ea4 | Oracle ID: 9c9b1871-e776-4373-ba89-e009131cb6f7
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::AMPHIBIOUS_KAVU,
    oracle_id = "9c9b1871-e776-4373-ba89-e009131cb6f7",
    scryfall_id = "37d94fb2-958c-487e-9f64-52d2771c6ea4",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Amphibious Kavu",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::KAVU],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
