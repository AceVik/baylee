//! Riptide Survivor — {2}{U} — Creature — Human Wizard
//! Oracle: Morph {1}{U}{U} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Oracle: When this creature is turned face up, discard two cards, then draw three cards.
//! Set: C14 #121 — Commander 2014 | Scryfall ID: cedabc94-2ed9-4a1e-a4d3-d004da959380 | Oracle ID: 10a64ae9-919f-4c04-8c8a-cce12554502c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RIPTIDE_SURVIVOR,
    oracle_id = "10a64ae9-919f-4c04-8c8a-cce12554502c",
    scryfall_id = "cedabc94-2ed9-4a1e-a4d3-d004da959380",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Riptide Survivor",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(2),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
