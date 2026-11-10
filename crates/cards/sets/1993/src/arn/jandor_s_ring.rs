//! Jandor's Ring — {6} — Artifact
//! Oracle: {2}, {T}, Discard the last card you drew this turn: Draw a card.
//! Set: SUM #256 — Summer Magic / Edgar | Scryfall ID: af9988fd-72a0-42dc-9f10-85edd3133977 | Oracle ID: 737db899-dcdb-48f9-8d30-cbbce3ae3434
// IMPLEMENTED — {2}, {T}, discard the last card drawn this turn: draw a card.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::JANDOR_S_RING,
    oracle_id = "737db899-dcdb-48f9-8d30-cbbce3ae3434",
    scryfall_id = "af9988fd-72a0-42dc-9f10-85edd3133977",
    faces = &[face!(
        name = "Jandor's Ring",
        mana_cost = mana!("{6}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        cost!("{2}", TapSelf, DiscardLastDrawnThisTurn),
        &[Effect::draw(1)]
    )],
);
