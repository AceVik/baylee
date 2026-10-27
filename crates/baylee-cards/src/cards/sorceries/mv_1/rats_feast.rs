//! Rats' Feast — {X}{B} — Sorcery
//! Oracle: Exile X target cards from a single graveyard.
//! Set: JUD #71 — Judgment | Scryfall ID: b243ce02-4fff-444c-acc4-e1a199621a53 | Oracle ID: 327c5ca2-4eeb-49cb-84fb-f6a4b06b24bc
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RATS_FEAST,
    oracle_id = "327c5ca2-4eeb-49cb-84fb-f6a4b06b24bc",
    scryfall_id = "b243ce02-4fff-444c-acc4-e1a199621a53",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Rats' Feast",
        mana_cost = mana!("{X}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
