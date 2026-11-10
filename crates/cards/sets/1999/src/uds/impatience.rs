//! Impatience — {2}{R} — Enchantment
//! Oracle: At the beginning of each player's end step, if that player didn't cast a spell this turn, this enchantment deals 2 damage to that player.
//! Set: 7ED #197 — Seventh Edition | Scryfall ID: 11443908-358f-4886-a2da-9b98002a3a3a | Oracle ID: 97c8f5e1-9fc1-4a62-a757-1c0454754cd3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::IMPATIENCE,
    oracle_id = "97c8f5e1-9fc1-4a62-a757-1c0454754cd3",
    scryfall_id = "11443908-358f-4886-a2da-9b98002a3a3a",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Impatience",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
