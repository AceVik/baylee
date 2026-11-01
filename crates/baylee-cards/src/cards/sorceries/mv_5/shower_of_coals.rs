//! Shower of Coals — {3}{R}{R} — Sorcery
//! Oracle: Shower of Coals deals 2 damage to each of up to three targets.
//! Oracle: Threshold — Shower of Coals deals 4 damage to each of those permanents and/or players instead if there are seven or more cards in your graveyard.
//! Set: ODY #221 — Odyssey | Scryfall ID: b1e623a8-145e-4649-b2cc-d3bb8a93f0f3 | Oracle ID: 08261613-1af3-4bd7-9a4d-a366fd508d5f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SHOWER_OF_COALS,
    oracle_id = "08261613-1af3-4bd7-9a4d-a366fd508d5f",
    scryfall_id = "b1e623a8-145e-4649-b2cc-d3bb8a93f0f3",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Shower of Coals",
        mana_cost = mana!("{3}{R}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
