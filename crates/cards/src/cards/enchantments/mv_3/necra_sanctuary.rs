//! Necra Sanctuary — {2}{B} — Enchantment
//! Oracle: At the beginning of your upkeep, if you control a green or white permanent, target player loses 1 life. If you control a green permanent and a white permanent, that player loses 3 life instead.
//! Set: APC #45 — Apocalypse | Scryfall ID: 5a0bf165-d7eb-4ae6-b30a-4e9fd55f401d | Oracle ID: 2586a59d-8501-4c22-9d69-f4bf91de7024
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NECRA_SANCTUARY,
    oracle_id = "2586a59d-8501-4c22-9d69-f4bf91de7024",
    scryfall_id = "5a0bf165-d7eb-4ae6-b30a-4e9fd55f401d",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Necra Sanctuary",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
