//! Overlaid Terrain — {2}{G}{G} — Enchantment
//! Oracle: As this enchantment enters, sacrifice all lands you control.
//! Oracle: Lands you control have "{T}: Add two mana of any one color."
//! Set: NEM #108 — Nemesis | Scryfall ID: 230c7926-9a4b-4ead-b4c8-889f84210545 | Oracle ID: e5818ab3-8c37-4063-9e9d-cbe4ae0bba14
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::OVERLAID_TERRAIN,
    oracle_id = "e5818ab3-8c37-4063-9e9d-cbe4ae0bba14",
    scryfall_id = "230c7926-9a4b-4ead-b4c8-889f84210545",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Overlaid Terrain",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
