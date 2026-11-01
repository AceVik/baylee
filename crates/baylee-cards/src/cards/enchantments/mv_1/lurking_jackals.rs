//! Lurking Jackals — {B} — Enchantment
//! Oracle: When an opponent has 10 or less life, if this permanent is an enchantment, it becomes a 3/2 Jackal creature.
//! Set: UDS #62 — Urza's Destiny | Scryfall ID: 97d082d8-f401-47ad-845c-77776ee647ba | Oracle ID: e9e5e17a-95fc-41b7-af4f-aec4802960a0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LURKING_JACKALS,
    oracle_id = "e9e5e17a-95fc-41b7-af4f-aec4802960a0",
    scryfall_id = "97d082d8-f401-47ad-845c-77776ee647ba",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Lurking Jackals",
        mana_cost = mana!("{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
