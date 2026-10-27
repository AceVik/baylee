//! Escape Artist — {1}{U} — Creature — Human Wizard
//! Oracle: This creature can't be blocked.
//! Oracle: {U}, Discard a card: Return this creature to its owner's hand.
//! Set: ODY #84 — Odyssey | Scryfall ID: 7f5d0e3f-b8f1-472a-857b-5464174d243b | Oracle ID: 74f4d505-e114-47be-be27-a7949e8abdc7
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ESCAPE_ARTIST,
    oracle_id = "74f4d505-e114-47be-be27-a7949e8abdc7",
    scryfall_id = "7f5d0e3f-b8f1-472a-857b-5464174d243b",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Escape Artist",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
