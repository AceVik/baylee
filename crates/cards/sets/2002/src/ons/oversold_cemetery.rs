//! Oversold Cemetery — {1}{B} — Enchantment
//! Oracle: At the beginning of your upkeep, if you have four or more creature cards in your graveyard, you may return target creature card from your graveyard to your hand.
//! Set: DMR #96 — Dominaria Remastered | Scryfall ID: 06b7312a-c775-4fa8-ba62-c84ff38459e8 | Oracle ID: ed4cd3a1-688b-4c05-948d-39d3336e00c0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::OVERSOLD_CEMETERY,
    oracle_id = "ed4cd3a1-688b-4c05-948d-39d3336e00c0",
    scryfall_id = "06b7312a-c775-4fa8-ba62-c84ff38459e8",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Oversold Cemetery",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
