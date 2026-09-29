//! Power Surge — {R}{R} — Enchantment
//! Oracle: At the beginning of each player's upkeep, this enchantment deals X damage to that player, where X is the number of untapped lands they controlled at the beginning of this turn.
//! Set: 4ED #216 — Fourth Edition | Scryfall ID: 0b5717af-a1a3-45cb-8b05-7543eed5532a | Oracle ID: 156b2228-f7b2-4816-b894-c4953a32c05f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::POWER_SURGE,
    oracle_id = "156b2228-f7b2-4816-b894-c4953a32c05f",
    scryfall_id = "0b5717af-a1a3-45cb-8b05-7543eed5532a",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Power Surge",
        mana_cost = mana!("{R}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
