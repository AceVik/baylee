//! Aphetto Dredging — {3}{B} — Sorcery
//! Oracle: Return up to three target creature cards of the creature type of your choice from your graveyard to your hand.
//! Set: ONS #125 — Onslaught | Scryfall ID: c4e7fadf-40f1-45ff-97ef-5830381accc9 | Oracle ID: 6cb8c627-6b2f-48a7-b2d9-8772defea560
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::APHETTO_DREDGING,
    oracle_id = "6cb8c627-6b2f-48a7-b2d9-8772defea560",
    scryfall_id = "c4e7fadf-40f1-45ff-97ef-5830381accc9",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Aphetto Dredging",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
