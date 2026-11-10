//! Cabal Executioner — {2}{B}{B} — Creature — Human Cleric
//! Oracle: Whenever this creature deals combat damage to a player, that player sacrifices a creature of their choice.
//! Oracle: Morph {3}{B}{B} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: ONS #130 — Onslaught | Scryfall ID: cd7727a7-0cdf-4fd5-82b4-e6587c10ca80 | Oracle ID: b7817579-df4d-41db-bb90-2aabac58f299
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CABAL_EXECUTIONER,
    oracle_id = "b7817579-df4d-41db-bb90-2aabac58f299",
    scryfall_id = "cd7727a7-0cdf-4fd5-82b4-e6587c10ca80",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Cabal Executioner",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
