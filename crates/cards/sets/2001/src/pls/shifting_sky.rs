//! Shifting Sky — {2}{U} — Enchantment
//! Oracle: As this enchantment enters, choose a color.
//! Oracle: All nonland permanents are the chosen color.
//! Set: 8ED #100 — Eighth Edition | Scryfall ID: fed441d0-b8c8-4d63-80ff-cc4fb47d0b26 | Oracle ID: 4634fd4e-5b38-4cfb-ab73-bf3b14b3bdee
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SHIFTING_SKY,
    oracle_id = "4634fd4e-5b38-4cfb-ab73-bf3b14b3bdee",
    scryfall_id = "fed441d0-b8c8-4d63-80ff-cc4fb47d0b26",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Shifting Sky",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
