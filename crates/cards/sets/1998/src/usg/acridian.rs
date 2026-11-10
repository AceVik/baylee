//! Acridian — {1}{G} — Creature — Insect
//! Oracle: Echo {1}{G} (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)
//! Set: USG #230 — Urza's Saga | Scryfall ID: 05d5a38f-5a60-46da-af1c-440e4bf7fe9e | Oracle ID: c6d0f1bc-b82a-4e64-a4bc-11ad3f54c71a
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ACRIDIAN,
    oracle_id = "c6d0f1bc-b82a-4e64-a4bc-11ad3f54c71a",
    scryfall_id = "05d5a38f-5a60-46da-af1c-440e4bf7fe9e",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Acridian",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::INSECT],
        power = Some(2),
        toughness = Some(4),
    ),],
    abilities = &[AbilityDef::Echo {
        cost: mana!("{1}{G}"),
    }],
);
