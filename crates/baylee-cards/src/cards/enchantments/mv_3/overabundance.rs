//! Overabundance — {1}{R}{G} — Enchantment
//! Oracle: Whenever a player taps a land for mana, that player adds one mana of any type that land produced, and this enchantment deals 1 damage to the player.
//! Set: INV #259 — Invasion | Scryfall ID: 4183e73d-609a-4292-b173-e39eb51949f3 | Oracle ID: b1ee0503-be92-4a06-89cd-5dfa937930d3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::OVERABUNDANCE,
    oracle_id = "b1ee0503-be92-4a06-89cd-5dfa937930d3",
    scryfall_id = "4183e73d-609a-4292-b173-e39eb51949f3",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red]),
    faces = &[face!(
        name = "Overabundance",
        mana_cost = mana!("{1}{R}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
