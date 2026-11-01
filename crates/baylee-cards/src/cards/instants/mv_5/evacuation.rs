//! Evacuation — {3}{U}{U} — Instant
//! Oracle: Return all creatures to their owners' hands.
//! Set: LCC #156 — The Lost Caverns of Ixalan Commander | Scryfall ID: b11037bc-e719-461e-a89e-9c026dfd9fd4 | Oracle ID: fdd94383-b573-439a-8e1c-925af887c5a6
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::EVACUATION,
    oracle_id = "fdd94383-b573-439a-8e1c-925af887c5a6",
    scryfall_id = "b11037bc-e719-461e-a89e-9c026dfd9fd4",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Evacuation",
        mana_cost = mana!("{3}{U}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[Effect::ReturnAllToHand {
        filter: &Filter::CREATURE,
        opponents_only: false,
    }])],
);
