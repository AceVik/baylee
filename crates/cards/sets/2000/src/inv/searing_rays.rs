//! Searing Rays — {2}{R} — Sorcery
//! Oracle: Choose a color. Searing Rays deals damage to each player equal to the number of creatures of that color that player controls.
//! Set: INV #165 — Invasion | Scryfall ID: 4f66ff2d-f2d2-4a6b-bf26-b510de60c0b6 | Oracle ID: fd6730e2-3904-43cf-90c6-e3d9fe210bc5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SEARING_RAYS,
    oracle_id = "fd6730e2-3904-43cf-90c6-e3d9fe210bc5",
    scryfall_id = "4f66ff2d-f2d2-4a6b-bf26-b510de60c0b6",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Searing Rays",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
