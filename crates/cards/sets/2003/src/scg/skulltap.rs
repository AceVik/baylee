//! Skulltap — {1}{B} — Sorcery
//! Oracle: As an additional cost to cast this spell, sacrifice a creature.
//! Oracle: Draw two cards.
//! Set: SCG #73 — Scourge | Scryfall ID: 48a90779-008e-401f-9877-be0a935d2ccd | Oracle ID: 17c7878f-891f-40a6-80a7-47fcc7b8e29b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SKULLTAP,
    oracle_id = "17c7878f-891f-40a6-80a7-47fcc7b8e29b",
    scryfall_id = "48a90779-008e-401f-9877-be0a935d2ccd",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Skulltap",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
