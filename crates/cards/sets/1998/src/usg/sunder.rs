//! Sunder — {3}{U}{U} — Instant
//! Oracle: Return all lands to their owners' hands.
//! Set: USG #101 — Urza's Saga | Scryfall ID: cd9dd7c6-36b6-4fe2-b3d3-f62a6e10a428 | Oracle ID: fddc93a2-912e-402a-9d0e-34f778e659ea
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SUNDER,
    oracle_id = "fddc93a2-912e-402a-9d0e-34f778e659ea",
    scryfall_id = "cd9dd7c6-36b6-4fe2-b3d3-f62a6e10a428",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Sunder",
        mana_cost = mana!("{3}{U}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[Effect::ReturnAllToHand {
        filter: &Filter::LAND,
        opponents_only: false,
    }]),],
);
