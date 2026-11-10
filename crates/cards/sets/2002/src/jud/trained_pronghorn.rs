//! Trained Pronghorn — {1}{W} — Creature — Antelope
//! Oracle: Discard a card: Prevent all damage that would be dealt to this creature this turn.
//! Set: JUD #30 — Judgment | Scryfall ID: 720ec745-226c-4211-974f-e04a4f9e1902 | Oracle ID: 22528380-4f47-4bd9-a1d4-79f700cba4f5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TRAINED_PRONGHORN,
    oracle_id = "22528380-4f47-4bd9-a1d4-79f700cba4f5",
    scryfall_id = "720ec745-226c-4211-974f-e04a4f9e1902",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Trained Pronghorn",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ANTELOPE],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
