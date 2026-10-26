//! Void — {3}{B}{R} — Sorcery
//! Oracle: Choose a number. Destroy all artifacts and creatures with mana value equal to that number. Then target player reveals their hand and discards all nonland cards with mana value equal to the number.
//! Set: EMA #211 — Eternal Masters | Scryfall ID: c0b3e320-a85c-4d92-944e-0a5e78a066a5 | Oracle ID: fdb28fd3-70fb-43c3-b367-9cf0d2c46e06
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::VOID,
    oracle_id = "fdb28fd3-70fb-43c3-b367-9cf0d2c46e06",
    scryfall_id = "c0b3e320-a85c-4d92-944e-0a5e78a066a5",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    faces = &[face!(
        name = "Void",
        mana_cost = mana!("{3}{B}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
