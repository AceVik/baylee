//! Tarrian's Journal // The Tomb of Aclazotz — {1}{B} — Legendary Artifact — Book // Legendary Land — Cave
//! Set: LCI #126 — The Lost Caverns of Ixalan | Scryfall ID: 99255a66-b868-45fc-a2a9-0c89bd851b69 | Oracle ID: a75b02ba-b0c8-47e3-a05c-e9ba221a7578
//! Face: Tarrian's Journal — {1}{B} — Legendary Artifact — Book
//! Face: The Tomb of Aclazotz —  — Legendary Land — Cave
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 1142,
    oracle_id: "a75b02ba-b0c8-47e3-a05c-e9ba221a7578",
    scryfall_id: "99255a66-b868-45fc-a2a9-0c89bd851b69",
    color_identity: ColorSet::from_slice(&[Color::Black]),
    faces: &[
    face! {
        name: "Tarrian's Journal",
        mana_cost: baylee_core::mana!("{1}{B}"),
        types: TypeSet::ARTIFACT,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[subtypes::artifact::BOOK],
    },
    face! {
        name: "The Tomb of Aclazotz",
        types: TypeSet::LAND,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[subtypes::land::CAVE],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
