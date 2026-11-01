//! Rhystic Scrying — {2}{U}{U} — Sorcery
//! Oracle: Draw three cards. Then if any player pays {2}, discard three cards.
//! Set: PCY #44 — Prophecy | Scryfall ID: 81a59737-f06f-49b7-a490-3dc1115b47b7 | Oracle ID: 7a07971a-bfc3-4d36-ad8a-b9d395cbd6e9
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RHYSTIC_SCRYING,
    oracle_id = "7a07971a-bfc3-4d36-ad8a-b9d395cbd6e9",
    scryfall_id = "81a59737-f06f-49b7-a490-3dc1115b47b7",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Rhystic Scrying",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
