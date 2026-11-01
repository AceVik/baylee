//! Maro — {2}{G}{G} — Creature — Elemental
//! Oracle: Maro's power and toughness are each equal to the number of cards in your hand.
//! Set: 9ED #254 — Ninth Edition | Scryfall ID: 9159e250-4eec-4ec4-80e1-42259331b3b9 | Oracle ID: ef32258a-d567-4fbc-ab85-8c00080f5b36
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MARO,
    oracle_id = "ef32258a-d567-4fbc-ab85-8c00080f5b36",
    scryfall_id = "9159e250-4eec-4ec4-80e1-42259331b3b9",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Maro",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELEMENTAL],
        power = Some(0),
        toughness = Some(0),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
