//! Krosan Colossus — {6}{G}{G}{G} — Creature — Beast
//! Oracle: Morph {6}{G}{G} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: MKC #176 — Murders at Karlov Manor Commander | Scryfall ID: 4f3c1e1a-bd75-4bfb-b44b-05b94f64c634 | Oracle ID: 703ab192-7e0a-4f02-8381-568ad2d999ca
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KROSAN_COLOSSUS,
    oracle_id = "703ab192-7e0a-4f02-8381-568ad2d999ca",
    scryfall_id = "4f3c1e1a-bd75-4bfb-b44b-05b94f64c634",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Krosan Colossus",
        mana_cost = mana!("{6}{G}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(9),
        toughness = Some(9),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
