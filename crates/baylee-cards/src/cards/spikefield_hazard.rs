//! Spikefield Hazard // Spikefield Cave — {R} — Instant // Land
//! Set: ZNR #166 — Zendikar Rising | Scryfall ID: a69541db-3f4e-412f-aa8e-dec1e74f74dc | Oracle ID: 81036c9f-fe0a-45a7-bcd5-0d344f31055a
//! Face: Spikefield Hazard — {R} — Instant
//! Face: Spikefield Cave —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 1074,
    oracle_id: "81036c9f-fe0a-45a7-bcd5-0d344f31055a",
    scryfall_id: "a69541db-3f4e-412f-aa8e-dec1e74f74dc",
    color_identity: ColorSet::from_slice(&[Color::Red]),
    faces: &[
    face! {
        name: "Spikefield Hazard",
        mana_cost: baylee_core::mana!("{R}"),
        types: TypeSet::INSTANT,
    },
    face! {
        name: "Spikefield Cave",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
