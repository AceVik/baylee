//! Dermoplasm — {2}{U} — Creature — Shapeshifter
//! Oracle: Flying
//! Oracle: Morph {2}{U}{U} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Oracle: When this creature is turned face up, you may put a creature card with a morph ability from your hand onto the battlefield face up. If you do, return this creature to its owner's hand.
//! Set: LGN #35 — Legions | Scryfall ID: cf2f5dca-e01f-41e3-bb6f-a60162118c6d | Oracle ID: 963d4712-51b2-4102-9626-d810e4696b70
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DERMOPLASM,
    oracle_id = "963d4712-51b2-4102-9626-d810e4696b70",
    scryfall_id = "cf2f5dca-e01f-41e3-bb6f-a60162118c6d",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Dermoplasm",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SHAPESHIFTER],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
