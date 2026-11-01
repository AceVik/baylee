//! Dematerialize — {3}{U} — Sorcery
//! Oracle: Return target permanent to its owner's hand.
//! Oracle: Flashback {5}{U}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: ODY #81 — Odyssey | Scryfall ID: 04217c17-7c29-4b02-b9b6-bfa1df50d4bc | Oracle ID: d2fbad7d-d05a-4321-9aff-a93bd102a3f4
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DEMATERIALIZE,
    oracle_id = "d2fbad7d-d05a-4321-9aff-a93bd102a3f4",
    scryfall_id = "04217c17-7c29-4b02-b9b6-bfa1df50d4bc",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Dematerialize",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
