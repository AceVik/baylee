//! Thoughtlace — {U} — Instant
//! Oracle: Target spell or permanent becomes blue. (Mana symbols on that permanent remain unchanged.)
//! Set: 4ED #107 — Fourth Edition | Scryfall ID: 185674a4-db97-444d-b0e9-7dcfc245ce4b | Oracle ID: 6452b6a6-6235-46a3-a712-a26592450438
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::THOUGHTLACE,
    oracle_id = "6452b6a6-6235-46a3-a712-a26592450438",
    scryfall_id = "185674a4-db97-444d-b0e9-7dcfc245ce4b",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Thoughtlace",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
