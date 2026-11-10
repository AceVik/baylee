//! Chance Encounter — {2}{R}{R} — Enchantment
//! Oracle: Whenever you win a coin flip, put a luck counter on this enchantment.
//! Oracle: At the beginning of your upkeep, if this enchantment has ten or more luck counters on it, you win the game.
//! Set: MH2 #277 — Modern Horizons 2 | Scryfall ID: 49a4b0c9-a35b-4b55-ab27-7246bbca0d16 | Oracle ID: 8bd33b5c-0777-41ef-9e17-fb30ce69ec25
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CHANCE_ENCOUNTER,
    oracle_id = "8bd33b5c-0777-41ef-9e17-fb30ce69ec25",
    scryfall_id = "49a4b0c9-a35b-4b55-ab27-7246bbca0d16",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Chance Encounter",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
