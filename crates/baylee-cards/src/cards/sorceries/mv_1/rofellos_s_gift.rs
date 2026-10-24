//! Rofellos's Gift — {G} — Sorcery
//! Oracle: Reveal any number of green cards in your hand. Return an enchantment card from your graveyard to your hand for each card revealed this way.
//! Set: UDS #119 — Urza's Destiny | Scryfall ID: a41347ba-b2e3-4d7e-8018-e6fd30243559 | Oracle ID: 7c309759-e607-4bad-b615-f7c673e508ac
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ROFELLOS_S_GIFT,
    oracle_id = "7c309759-e607-4bad-b615-f7c673e508ac",
    scryfall_id = "a41347ba-b2e3-4d7e-8018-e6fd30243559",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Rofellos's Gift",
        mana_cost = mana!("{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
