//! Sanctum Guardian — {1}{W}{W} — Creature — Human Cleric
//! Oracle: Sacrifice this creature: The next time a source of your choice would deal damage to any target this turn, prevent that damage.
//! Set: USG #43 — Urza's Saga | Scryfall ID: 0d09137f-0f4c-4389-a915-0ce02c833d94 | Oracle ID: eb7fe094-2237-40c8-b831-93498c93b768
// PARTIAL — choosing a damage source and preventing its next damage to any target is not supported; retains only its creature body

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SANCTUM_GUARDIAN,
    oracle_id = "eb7fe094-2237-40c8-b831-93498c93b768",
    scryfall_id = "0d09137f-0f4c-4389-a915-0ce02c833d94",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "choosing a damage source and preventing its next damage to any target is not supported; retains only its creature body",
    ),
    faces = &[face!(
        name = "Sanctum Guardian",
        mana_cost = mana!("{1}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(4),
    ),],
    // NOT SUPPORTED: choosing a damage source and preventing its next damage to any target is not supported; retains only its creature body.
    abilities = &[],
);
