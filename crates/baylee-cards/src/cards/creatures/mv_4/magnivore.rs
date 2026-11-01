//! Magnivore — {2}{R}{R} — Creature — Lhurgoyf
//! Oracle: Haste (This creature can attack and {T} as soon as it comes under your control.)
//! Oracle: Magnivore's power and toughness are each equal to the number of sorcery cards in all graveyards.
//! Set: 9ED #202 — Ninth Edition | Scryfall ID: ab077f67-09c3-43d0-ad27-d1e6255c4d55 | Oracle ID: 882da1c6-7b86-4de0-9161-ce7c50f9c90c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MAGNIVORE,
    oracle_id = "882da1c6-7b86-4de0-9161-ce7c50f9c90c",
    scryfall_id = "ab077f67-09c3-43d0-ad27-d1e6255c4d55",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Magnivore",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::LHURGOYF],
        power = Some(0),
        toughness = Some(0),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
