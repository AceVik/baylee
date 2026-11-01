//! Titanic Bulvox — {6}{G}{G} — Creature — Beast
//! Oracle: Trample
//! Oracle: Morph {4}{G}{G}{G} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: SCG #129 — Scourge | Scryfall ID: 3f42c4d7-b555-449c-a539-119c1ae62232 | Oracle ID: 2cc63297-7eb9-422e-838c-a9260756d689
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TITANIC_BULVOX,
    oracle_id = "2cc63297-7eb9-422e-838c-a9260756d689",
    scryfall_id = "3f42c4d7-b555-449c-a539-119c1ae62232",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Titanic Bulvox",
        mana_cost = mana!("{6}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(7),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
