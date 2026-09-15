//! Tarrian's Journal // The Tomb of Aclazotz — {1}{B} — Legendary Artifact — Book // Legendary Land — Cave
//! Oracle: {T}, Sacrifice another artifact or creature: Draw a card. Activate only as a sorcery.
//! Oracle: {2}, {T}, Discard your hand: Transform Tarrian's Journal.
//! Oracle: (Transforms from Tarrian's Journal.)
//! Oracle: {T}: Add {B}.
//! Oracle: {T}: You may cast a creature spell from your graveyard this turn. If you do, it enters with a finality counter on it and is a Vampire in addition to its other types. (If a creature with a finality counter on it would die, exile it instead.)
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
        mana_cost: mana!("{1}{B}"),
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
