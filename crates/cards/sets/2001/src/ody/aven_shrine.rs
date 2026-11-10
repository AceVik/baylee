//! Aven Shrine — {1}{W}{W} — Enchantment
//! Oracle: Whenever a player casts a spell, that player gains X life, where X is the number of cards in all graveyards with the same name as that spell.
//! Set: ODY #9 — Odyssey | Scryfall ID: 9890be7f-35af-43b7-b255-25be4ff20dc0 | Oracle ID: 09c450cf-5f22-45c2-86ed-21b85552a96f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::AVEN_SHRINE,
    oracle_id = "09c450cf-5f22-45c2-86ed-21b85552a96f",
    scryfall_id = "9890be7f-35af-43b7-b255-25be4ff20dc0",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Aven Shrine",
        mana_cost = mana!("{1}{W}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
