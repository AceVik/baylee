//! Verdant Succession — {4}{G} — Enchantment
//! Oracle: Whenever a green nontoken creature dies, that creature's controller may search their library for a card with the same name as that creature, put it onto the battlefield, then shuffle.
//! Set: ODY #280 — Odyssey | Scryfall ID: bd3c78de-0c2a-441e-8912-ff920fc563ef | Oracle ID: 1440ec54-067e-479b-9196-edf84534a04e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::VERDANT_SUCCESSION,
    oracle_id = "1440ec54-067e-479b-9196-edf84534a04e",
    scryfall_id = "bd3c78de-0c2a-441e-8912-ff920fc563ef",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Verdant Succession",
        mana_cost = mana!("{4}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
