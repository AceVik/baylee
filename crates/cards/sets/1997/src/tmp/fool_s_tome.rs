//! Fool's Tome — {4} — Artifact — Book
//! Oracle: {2}, {T}: Draw a card. Activate only if you have no cards in hand.
//! Set: TMP #289 — Tempest | Scryfall ID: 83be257c-8945-46be-8b58-fb2881084026 | Oracle ID: 65589538-d3c1-4dfd-a328-71f57f556212
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FOOL_S_TOME,
    oracle_id = "65589538-d3c1-4dfd-a328-71f57f556212",
    scryfall_id = "83be257c-8945-46be-8b58-fb2881084026",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Fool's Tome",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::BOOK],
    ),],
    abilities = &[activated!(
        cost!("{2}", TapSelf),
        &[Effect::draw(1)],
        condition = Some(Condition::HandSizeAtMost(0)),
    )],
);
