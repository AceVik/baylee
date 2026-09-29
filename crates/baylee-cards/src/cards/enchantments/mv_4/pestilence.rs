//! Pestilence — {2}{B}{B} — Enchantment
//! Oracle: At the beginning of the end step, if no creatures are on the battlefield, sacrifice this enchantment.
//! Oracle: {B}: This enchantment deals 1 damage to each creature and each player.
//! Set: 6ED #149 — Classic Sixth Edition | Scryfall ID: 29d852c4-bd53-4a3b-b1e2-896917cbc27f | Oracle ID: dafe63ef-f3d6-45e7-877a-573da92ba85e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PESTILENCE,
    oracle_id = "dafe63ef-f3d6-45e7-877a-573da92ba85e",
    scryfall_id = "29d852c4-bd53-4a3b-b1e2-896917cbc27f",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Pestilence",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
