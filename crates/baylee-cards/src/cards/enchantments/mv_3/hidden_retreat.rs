//! Hidden Retreat — {2}{W} — Enchantment
//! Oracle: Put a card from your hand on top of your library: Prevent all damage that would be dealt by target instant or sorcery spell this turn.
//! Set: STH #6 — Stronghold | Scryfall ID: 5be86f1f-c85a-4396-aa16-a39fbf493c96 | Oracle ID: a116329a-343e-4f10-a122-38bf8b5ac2c8
// PARTIAL: casts and enters as an enchantment with no abilities; putting a card from your hand on top of your library as an activation cost to prevent all damage from target instant or sorcery spell is not supported

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HIDDEN_RETREAT,
    oracle_id = "a116329a-343e-4f10-a122-38bf8b5ac2c8",
    scryfall_id = "5be86f1f-c85a-4396-aa16-a39fbf493c96",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "casts and enters as an enchantment with no abilities; putting a card from your hand on top of your library as an activation cost to prevent all damage from target instant or sorcery spell is not supported"
    ),
    faces = &[face!(
        name = "Hidden Retreat",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);
