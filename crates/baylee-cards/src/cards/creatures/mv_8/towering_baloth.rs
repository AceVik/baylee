//! Towering Baloth — {6}{G}{G} — Creature — Beast
//! Oracle: Morph {6}{G} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: ONS #292 — Onslaught | Scryfall ID: 2a8cc948-28ff-4bbe-b8c9-71de37478023 | Oracle ID: 3607d700-5a05-4e68-8863-3514c68566ef
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TOWERING_BALOTH,
    oracle_id = "3607d700-5a05-4e68-8863-3514c68566ef",
    scryfall_id = "2a8cc948-28ff-4bbe-b8c9-71de37478023",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Towering Baloth",
        mana_cost = mana!("{6}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(7),
        toughness = Some(6),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
