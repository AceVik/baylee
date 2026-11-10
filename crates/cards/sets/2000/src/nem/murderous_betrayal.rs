//! Murderous Betrayal — {B}{B}{B} — Enchantment
//! Oracle: {B}{B}, Pay half your life, rounded up: Destroy target nonblack creature. It can't be regenerated.
//! Set: 8ED #147 — Eighth Edition | Scryfall ID: 0063ed19-494b-4199-9120-d479bfcb625a | Oracle ID: a2fe1e2b-b621-4406-b5e0-f2eebd12c2ba
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MURDEROUS_BETRAYAL,
    oracle_id = "a2fe1e2b-b621-4406-b5e0-f2eebd12c2ba",
    scryfall_id = "0063ed19-494b-4199-9120-d479bfcb625a",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Murderous Betrayal",
        mana_cost = mana!("{B}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
