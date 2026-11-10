//! Balshan Beguiler — {2}{U} — Creature — Human Wizard
//! Oracle: Whenever this creature deals combat damage to a player, that player reveals the top two cards of their library. You choose one of those cards and put it into their graveyard.
//! Set: ODY #66 — Odyssey | Scryfall ID: 5d977da2-4024-4c7b-b557-e89564f8d465 | Oracle ID: 928a7fbc-ba34-42a9-947e-0f3bf1cba152
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BALSHAN_BEGUILER,
    oracle_id = "928a7fbc-ba34-42a9-947e-0f3bf1cba152",
    scryfall_id = "5d977da2-4024-4c7b-b557-e89564f8d465",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Balshan Beguiler",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
