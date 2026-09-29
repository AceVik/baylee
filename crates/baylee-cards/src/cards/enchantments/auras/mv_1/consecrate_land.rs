//! Consecrate Land — {W} — Enchantment — Aura
//! Oracle: Enchant land
//! Oracle: Enchanted land has indestructible and can't be enchanted by other Auras.
//! Set: TSB #4 — Time Spiral Timeshifted | Scryfall ID: ded79afb-2a65-49e8-81c3-757e5d4c2203 | Oracle ID: 4627691c-4ed4-4add-9cc3-2e019be2f9fd
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CONSECRATE_LAND,
    oracle_id = "4627691c-4ed4-4add-9cc3-2e019be2f9fd",
    scryfall_id = "ded79afb-2a65-49e8-81c3-757e5d4c2203",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Consecrate Land",
        mana_cost = mana!("{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
