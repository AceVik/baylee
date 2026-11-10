//! Daru Healer — {2}{W} — Creature — Human Cleric
//! Oracle: {T}: Prevent the next 1 damage that would be dealt to any target this turn.
//! Oracle: Morph {W} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: ONS #19 — Onslaught | Scryfall ID: 0e4f3eff-ac99-41e2-9003-9630cdb3ae23 | Oracle ID: 630b1685-d0c9-4d94-97dc-70058fb52e5d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DARU_HEALER,
    oracle_id = "630b1685-d0c9-4d94-97dc-70058fb52e5d",
    scryfall_id = "0e4f3eff-ac99-41e2-9003-9630cdb3ae23",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Daru Healer",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
