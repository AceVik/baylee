//! Plague Spores — {4}{B}{R} — Sorcery
//! Oracle: Destroy target nonblack creature and target land. They can't be regenerated.
//! Set: INV #260 — Invasion | Scryfall ID: 0d106d56-a688-49cc-8d5d-0279a5a7c0a7 | Oracle ID: f79f5cb7-d238-41ab-85f0-eb21cc6bf561
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PLAGUE_SPORES,
    oracle_id = "f79f5cb7-d238-41ab-85f0-eb21cc6bf561",
    scryfall_id = "0d106d56-a688-49cc-8d5d-0279a5a7c0a7",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    faces = &[face!(
        name = "Plague Spores",
        mana_cost = mana!("{4}{B}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
