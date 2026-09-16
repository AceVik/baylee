//! Thassa's Oracle — {U}{U} — Creature — Merfolk Wizard
//! Oracle: When this creature enters, look at the top X cards of your library, where X is your devotion to blue. Put up to one of them on top of your library and the rest on the bottom of your library in a random order. If X is greater than or equal to the number of cards in your library, you win the game. (Each {U} in the mana costs of permanents you control counts toward your devotion to blue.)
//! Set: THB #73 — Theros Beyond Death | Scryfall ID: 726e8b29-13e9-4138-b6a9-d2a0d8188d1c | Oracle ID: 1de1b591-a73f-4974-b507-8c63e07a0868
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THASSA_S_ORACLE,
    oracle_id = "1de1b591-a73f-4974-b507-8c63e07a0868",
    scryfall_id = "726e8b29-13e9-4138-b6a9-d2a0d8188d1c",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Thassa's Oracle",
        mana_cost = mana!("{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::MERFOLK, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
