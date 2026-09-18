//! Risen Reef — {1}{G}{U} — Creature — Elemental
//! Oracle: Whenever this creature or another Elemental you control enters, look at the top card of your library. If it's a land card, you may put it onto the battlefield tapped. If you don't put the card onto the battlefield, put it into your hand.
//! Set: ECC #132 — Lorwyn Eclipsed Commander | Scryfall ID: 715ad2ff-7eae-42f1-bb3c-a3afc2c9b82a | Oracle ID: 2ae71e86-4400-4a30-9077-4d57a43e7395
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RISEN_REEF,
    oracle_id = "2ae71e86-4400-4a30-9077-4d57a43e7395",
    scryfall_id = "715ad2ff-7eae-42f1-bb3c-a3afc2c9b82a",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue]),
    faces = &[face!(
        name = "Risen Reef",
        mana_cost = mana!("{1}{G}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELEMENTAL],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
