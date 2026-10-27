//! Venomspout Brackus — {6}{G} — Creature — Beast
//! Oracle: {1}{G}, {T}: This creature deals 5 damage to target attacking or blocking creature with flying.
//! Oracle: Morph {3}{G}{G} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: ONS #295 — Onslaught | Scryfall ID: 0774771c-5373-4636-9174-d06e7d635183 | Oracle ID: e12f00f4-89f2-4d1d-94cf-460945d80a4e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VENOMSPOUT_BRACKUS,
    oracle_id = "e12f00f4-89f2-4d1d-94cf-460945d80a4e",
    scryfall_id = "0774771c-5373-4636-9174-d06e7d635183",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Venomspout Brackus",
        mana_cost = mana!("{6}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(5),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
