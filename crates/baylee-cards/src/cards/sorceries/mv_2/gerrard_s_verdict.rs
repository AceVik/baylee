//! Gerrard's Verdict — {W}{B} — Sorcery
//! Oracle: Target player discards two cards. You gain 3 life for each land card discarded this way.
//! Set: DMR #190 — Dominaria Remastered | Scryfall ID: e7d9d3be-0a66-4871-9d60-d253d3a37a13 | Oracle ID: 670b5dfb-ffdf-43ed-acc3-855006e352fc
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GERRARD_S_VERDICT,
    oracle_id = "670b5dfb-ffdf-43ed-acc3-855006e352fc",
    scryfall_id = "e7d9d3be-0a66-4871-9d60-d253d3a37a13",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::White]),
    faces = &[face!(
        name = "Gerrard's Verdict",
        mana_cost = mana!("{W}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
