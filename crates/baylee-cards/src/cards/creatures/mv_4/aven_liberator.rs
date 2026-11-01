//! Aven Liberator — {2}{W}{W} — Creature — Bird Soldier
//! Oracle: Flying
//! Oracle: Morph {3}{W} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Oracle: When this creature is turned face up, target creature you control gains protection from the color of your choice until end of turn.
//! Set: SCG #4 — Scourge | Scryfall ID: b2804006-2a60-400c-be0b-8aa042469372 | Oracle ID: 0f3fff7a-cc46-4c72-9edc-fdbddd3b51b5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::AVEN_LIBERATOR,
    oracle_id = "0f3fff7a-cc46-4c72-9edc-fdbddd3b51b5",
    scryfall_id = "b2804006-2a60-400c-be0b-8aa042469372",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Aven Liberator",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BIRD, subtypes::creature::SOLDIER],
        power = Some(2),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
