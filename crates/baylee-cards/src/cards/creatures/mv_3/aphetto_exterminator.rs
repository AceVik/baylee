//! Aphetto Exterminator — {2}{B} — Creature — Human Wizard
//! Oracle: Morph {3}{B} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Oracle: When this creature is turned face up, target creature gets -3/-3 until end of turn.
//! Set: LGN #59 — Legions | Scryfall ID: 06be8f63-daf2-4dbe-bb07-2b246145cdab | Oracle ID: a3bf5d18-77d5-4881-916c-cc125e621b6e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::APHETTO_EXTERMINATOR,
    oracle_id = "a3bf5d18-77d5-4881-916c-cc125e621b6e",
    scryfall_id = "06be8f63-daf2-4dbe-bb07-2b246145cdab",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Aphetto Exterminator",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(3),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
