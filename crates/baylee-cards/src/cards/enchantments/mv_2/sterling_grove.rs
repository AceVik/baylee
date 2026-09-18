//! Sterling Grove — {G}{W} — Enchantment
//! Oracle: Other enchantments you control have shroud. (They can't be the targets of spells or abilities.)
//! Oracle: {1}, Sacrifice this enchantment: Search your library for an enchantment card, reveal it, then shuffle and put that card on top.
//! Set: MH2 #293 — Modern Horizons 2 | Scryfall ID: ba03e105-a76c-4769-a35a-d780448890ec | Oracle ID: 2c275a85-5a15-46cd-a6e7-add63f9b853d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::STERLING_GROVE,
    oracle_id = "2c275a85-5a15-46cd-a6e7-add63f9b853d",
    scryfall_id = "ba03e105-a76c-4769-a35a-d780448890ec",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    faces = &[face!(
        name = "Sterling Grove",
        mana_cost = mana!("{G}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
