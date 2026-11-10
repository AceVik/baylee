//! Thought Devourer — {2}{U}{U} — Creature — Beast
//! Oracle: Flying
//! Oracle: Your maximum hand size is reduced by four.
//! Set: ODY #105 — Odyssey | Scryfall ID: ba7a96ee-e2d1-4d76-a09e-d6868ddd9282 | Oracle ID: e5106a2d-caca-4519-a1a1-eabaec4005df
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THOUGHT_DEVOURER,
    oracle_id = "e5106a2d-caca-4519-a1a1-eabaec4005df",
    scryfall_id = "ba7a96ee-e2d1-4d76-a09e-d6868ddd9282",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Thought Devourer",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
