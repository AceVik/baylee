//! Sevinne's Reclamation — {2}{W} — Sorcery
//! Oracle: Return target permanent card with mana value 3 or less from your graveyard to the battlefield. If this spell was cast from a graveyard, you may copy this spell and may choose a new target for the copy.
//! Oracle: Flashback {4}{W} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: SOC #170 — Secrets of Strixhaven Commander | Scryfall ID: 8deab1ef-4219-4767-a3c4-b61250d0ebe0 | Oracle ID: 1b9f9f5b-8712-4f00-90cb-1b7b9970eccc
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SEVINNE_S_RECLAMATION,
    oracle_id = "1b9f9f5b-8712-4f00-90cb-1b7b9970eccc",
    scryfall_id = "8deab1ef-4219-4767-a3c4-b61250d0ebe0",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Sevinne's Reclamation",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
