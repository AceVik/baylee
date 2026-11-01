//! Phyrexian Slayer — {3}{B} — Creature — Phyrexian Minion
//! Oracle: Flying
//! Oracle: Whenever this creature becomes blocked by a white creature, destroy that creature. It can't be regenerated.
//! Set: INV #118 — Invasion | Scryfall ID: 5fa8c604-343f-4c94-ac25-439ab1845c19 | Oracle ID: da74cb57-f6de-4d05-a339-6412beb8df25
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PHYREXIAN_SLAYER,
    oracle_id = "da74cb57-f6de-4d05-a339-6412beb8df25",
    scryfall_id = "5fa8c604-343f-4c94-ac25-439ab1845c19",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Phyrexian Slayer",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::PHYREXIAN, subtypes::creature::MINION],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
