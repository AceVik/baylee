//! Conversion — {2}{W}{W} — Enchantment
//! Oracle: At the beginning of your upkeep, sacrifice this enchantment unless you pay {W}{W}.
//! Oracle: All Mountains are Plains.
//! Set: ME4 #9 — Masters Edition IV | Scryfall ID: 70bad071-f01e-4309-b37a-a3ea45dc7d3a | Oracle ID: a24e05fb-dffb-4400-b4ca-22fdde45e7a7
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CONVERSION,
    oracle_id = "a24e05fb-dffb-4400-b4ca-22fdde45e7a7",
    scryfall_id = "70bad071-f01e-4309-b37a-a3ea45dc7d3a",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Conversion",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
