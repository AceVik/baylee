//! Great Whale — {5}{U}{U} — Creature — Whale
//! Oracle: When this creature enters, untap up to seven lands.
//! Set: USG #77 — Urza's Saga | Scryfall ID: 58a2acf1-dad8-4f93-a34e-891e5178a48f | Oracle ID: c43ce30a-0346-479d-8dbc-56c3872ff82a
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GREAT_WHALE,
    oracle_id = "c43ce30a-0346-479d-8dbc-56c3872ff82a",
    scryfall_id = "58a2acf1-dad8-4f93-a34e-891e5178a48f",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Great Whale",
        mana_cost = mana!("{5}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WHALE],
        power = Some(5),
        toughness = Some(5),
    ),],
    abilities = &[triggered!(
        Trigger::ETB,
        &[Effect::UntapChosen {
            filter: &Filter::LAND,
            count: 7,
        }],
    )],
);
