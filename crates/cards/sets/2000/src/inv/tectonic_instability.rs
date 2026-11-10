//! Tectonic Instability — {2}{R} — Enchantment
//! Oracle: Whenever a land enters, tap all lands its controller controls.
//! Set: INV #173 — Invasion | Scryfall ID: 0476cc6b-ecc6-44d6-9f44-a90d4ee85daa | Oracle ID: 0e68ee36-9649-490a-8483-fc8bdc23f161
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TECTONIC_INSTABILITY,
    oracle_id = "0e68ee36-9649-490a-8483-fc8bdc23f161",
    scryfall_id = "0476cc6b-ecc6-44d6-9f44-a90d4ee85daa",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Tectonic Instability",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
