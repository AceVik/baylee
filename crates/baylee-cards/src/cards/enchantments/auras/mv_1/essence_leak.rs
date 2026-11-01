//! Essence Leak — {U} — Enchantment — Aura
//! Oracle: Enchant permanent
//! Oracle: As long as enchanted permanent is red or green, it has "At the beginning of your upkeep, sacrifice this permanent unless you pay its mana cost."
//! Set: INV #55 — Invasion | Scryfall ID: 9099b2e6-9ed8-4a9c-97ca-77cc47678228 | Oracle ID: cb02daf6-9013-4ccc-bf1f-773b757ac468
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ESSENCE_LEAK,
    oracle_id = "cb02daf6-9013-4ccc-bf1f-773b757ac468",
    scryfall_id = "9099b2e6-9ed8-4a9c-97ca-77cc47678228",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Essence Leak",
        mana_cost = mana!("{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
