//! Underworld Breach — {1}{R} — Enchantment
//! Oracle: Each nonland card in your graveyard has escape. The escape cost is equal to the card's mana cost plus exile three other cards from your graveyard. (You may cast cards from your graveyard for their escape cost.)
//! Oracle: At the beginning of the end step, sacrifice this enchantment.
//! Set: THB #161 — Theros Beyond Death | Scryfall ID: 0e51d796-7279-4c06-87f0-37adbdaa41df | Oracle ID: 27e0948b-9916-473b-8d8c-a51bdfbc7457
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::UNDERWORLD_BREACH,
    oracle_id = "27e0948b-9916-473b-8d8c-a51bdfbc7457",
    scryfall_id = "0e51d796-7279-4c06-87f0-37adbdaa41df",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Underworld Breach",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
