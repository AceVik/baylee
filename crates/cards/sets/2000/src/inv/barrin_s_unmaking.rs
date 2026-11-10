//! Barrin's Unmaking — {1}{U} — Instant
//! Oracle: Return target permanent to its owner's hand if that permanent shares a color with the most common color among all permanents or a color tied for most common.
//! Set: INV #46 — Invasion | Scryfall ID: 4d4cecb0-12b5-4678-b5e7-8cec8fc86cef | Oracle ID: bd7ad427-11d9-40d1-b292-71f985711665
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BARRIN_S_UNMAKING,
    oracle_id = "bd7ad427-11d9-40d1-b292-71f985711665",
    scryfall_id = "4d4cecb0-12b5-4678-b5e7-8cec8fc86cef",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Barrin's Unmaking",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
