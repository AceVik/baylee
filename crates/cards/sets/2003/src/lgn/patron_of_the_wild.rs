//! Patron of the Wild — {G} — Creature — Elf
//! Oracle: Morph {2}{G} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Oracle: When this creature is turned face up, target creature gets +3/+3 until end of turn.
//! Set: LGN #134 — Legions | Scryfall ID: 7f7a0810-3970-454f-8381-700d6c6aefdc | Oracle ID: 7ba8ae5b-8025-40eb-969d-38a85eefe7d6
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PATRON_OF_THE_WILD,
    oracle_id = "7ba8ae5b-8025-40eb-969d-38a85eefe7d6",
    scryfall_id = "7f7a0810-3970-454f-8381-700d6c6aefdc",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Patron of the Wild",
        mana_cost = mana!("{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELF],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
