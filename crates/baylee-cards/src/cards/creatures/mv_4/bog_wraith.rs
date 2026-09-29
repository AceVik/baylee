//! Bog Wraith — {3}{B} — Creature — Wraith
//! Oracle: Swampwalk (This creature can't be blocked as long as defending player controls a Swamp.)
//! Set: M10 #86 — Magic 2010 | Scryfall ID: 5d17210c-be39-4abc-9d4d-4f2eca9573bd | Oracle ID: 508248d1-09a4-4e41-a4c9-286618e5061e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BOG_WRAITH,
    oracle_id = "508248d1-09a4-4e41-a4c9-286618e5061e",
    scryfall_id = "5d17210c-be39-4abc-9d4d-4f2eca9573bd",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Bog Wraith",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WRAITH],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
