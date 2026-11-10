//! Krosan Tusker — {5}{G}{G} — Creature — Boar Beast
//! Oracle: Cycling {2}{G} ({2}{G}, Discard this card: Draw a card.)
//! Oracle: When you cycle this card, you may search your library for a basic land card, reveal that card, put it into your hand, then shuffle. (Do this before you draw.)
//! Set: CMM #302 — Commander Masters | Scryfall ID: 0a368f72-cc88-4f32-aad4-22d92c4d032d | Oracle ID: 82406175-63b3-4972-be8b-db0e70c908cb
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KROSAN_TUSKER,
    oracle_id = "82406175-63b3-4972-be8b-db0e70c908cb",
    scryfall_id = "0a368f72-cc88-4f32-aad4-22d92c4d032d",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Krosan Tusker",
        mana_cost = mana!("{5}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BOAR, subtypes::creature::BEAST],
        power = Some(6),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
