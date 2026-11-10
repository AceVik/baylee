//! Conch Horn — {2} — Artifact
//! Oracle: {1}, {T}, Sacrifice this artifact: Draw two cards, then put a card from your hand on top of your library.
//! Set: FEM #83 — Fallen Empires | Scryfall ID: 860a9ba3-e4c4-4af9-bdfe-1ada39289fd5 | Oracle ID: b019cc20-d32d-4cdf-80cc-eafb3f78db88
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CONCH_HORN,
    oracle_id = "b019cc20-d32d-4cdf-80cc-eafb3f78db88",
    scryfall_id = "860a9ba3-e4c4-4af9-bdfe-1ada39289fd5",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Conch Horn",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{1}", TapSelf, SacrificeSelf),
        &[Effect::draw(2), Effect::PutFromHandOnTop { count: 1 }],
    ),],
);
