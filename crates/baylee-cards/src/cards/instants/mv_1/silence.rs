//! Silence — {W} — Instant
//! Oracle: Your opponents can't cast spells this turn.
//! Set: M14 #35 — Magic 2014 | Scryfall ID: 1c2b13b1-31f0-4676-88a7-53f3a190e9a2 | Oracle ID: 8aed54cb-d1bb-45ad-adbe-38e55d84ff31
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SILENCE,
    oracle_id = "8aed54cb-d1bb-45ad-adbe-38e55d84ff31",
    scryfall_id = "1c2b13b1-31f0-4676-88a7-53f3a190e9a2",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Silence",
        mana_cost = mana!("{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
