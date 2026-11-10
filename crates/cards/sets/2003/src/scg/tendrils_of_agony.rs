//! Tendrils of Agony — {2}{B}{B} — Sorcery
//! Oracle: Target player loses 2 life and you gain 2 life.
//! Oracle: Storm (When you cast this spell, copy it for each spell cast before it this turn. You may choose new targets for the copies.)
//! Set: VMA #142 — Vintage Masters | Scryfall ID: 6f26faca-f338-4ce5-a218-6a61d40fc50a | Oracle ID: 78db3190-59bc-4033-95bc-32d8ed872b6a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TENDRILS_OF_AGONY,
    oracle_id = "78db3190-59bc-4033-95bc-32d8ed872b6a",
    scryfall_id = "6f26faca-f338-4ce5-a218-6a61d40fc50a",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Tendrils of Agony",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
