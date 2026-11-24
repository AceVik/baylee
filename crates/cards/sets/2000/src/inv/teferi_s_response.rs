//! Teferi's Response — {1}{U} — Instant
//! Oracle: Counter target spell or ability an opponent controls that targets a land you control. If a permanent's ability is countered this way, destroy that permanent.
//! Oracle: Draw two cards.
//! Set: INV #78 — Invasion | Scryfall ID: f3bb2df8-c559-4a34-83b0-d48fbc694cc8 | Oracle ID: b2faa8b6-e433-4171-9774-9170484530c4
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TEFERI_S_RESPONSE,
    oracle_id = "b2faa8b6-e433-4171-9774-9170484530c4",
    scryfall_id = "f3bb2df8-c559-4a34-83b0-d48fbc694cc8",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Teferi's Response",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
