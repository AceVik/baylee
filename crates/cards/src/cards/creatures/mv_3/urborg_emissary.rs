//! Urborg Emissary — {2}{B} — Creature — Human Wizard
//! Oracle: Kicker {1}{U} (You may pay an additional {1}{U} as you cast this spell.)
//! Oracle: When this creature enters, if it was kicked, return target permanent to its owner's hand.
//! Set: INV #131 — Invasion | Scryfall ID: e6912c71-1836-4e87-9a65-d577d903d03c | Oracle ID: f30943f1-473f-4fa4-b6a7-3dfbd1995ba9
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::URBORG_EMISSARY,
    oracle_id = "f30943f1-473f-4fa4-b6a7-3dfbd1995ba9",
    scryfall_id = "e6912c71-1836-4e87-9a65-d577d903d03c",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    faces = &[face!(
        name = "Urborg Emissary",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(3),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
