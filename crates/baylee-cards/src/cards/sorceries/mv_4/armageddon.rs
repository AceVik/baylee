//! Armageddon — {3}{W} — Sorcery
//! Oracle: Destroy all lands.
//! Set: A25 #5 — Masters 25 | Scryfall ID: 77f1f6ac-983f-4f3e-8906-47f774e8367b | Oracle ID: c9ed8b01-959a-47d6-891e-0abbdccf6e4f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ARMAGEDDON,
    oracle_id = "c9ed8b01-959a-47d6-891e-0abbdccf6e4f",
    scryfall_id = "77f1f6ac-983f-4f3e-8906-47f774e8367b",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Armageddon",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
