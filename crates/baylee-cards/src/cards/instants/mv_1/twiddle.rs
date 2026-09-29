//! Twiddle — {U} — Instant
//! Oracle: You may tap or untap target artifact, creature, or land.
//! Set: 8ED #111 — Eighth Edition | Scryfall ID: 1b25858a-ab2d-441a-a3fe-6d5ecd7f05be | Oracle ID: 773ad2ef-5acc-49ea-8d85-056330e87039
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TWIDDLE,
    oracle_id = "773ad2ef-5acc-49ea-8d85-056330e87039",
    scryfall_id = "1b25858a-ab2d-441a-a3fe-6d5ecd7f05be",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Twiddle",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
