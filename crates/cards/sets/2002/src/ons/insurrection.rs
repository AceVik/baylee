//! Insurrection — {5}{R}{R}{R} — Sorcery
//! Oracle: Untap all creatures and gain control of them until end of turn. They gain haste until end of turn.
//! Set: CMM #236 — Commander Masters | Scryfall ID: df8a0a8c-1953-46e6-9da5-b4c20909ce1c | Oracle ID: 7f7c204f-be1a-47e4-91c6-ba6f906d9012
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::INSURRECTION,
    oracle_id = "7f7c204f-be1a-47e4-91c6-ba6f906d9012",
    scryfall_id = "df8a0a8c-1953-46e6-9da5-b4c20909ce1c",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Insurrection",
        mana_cost = mana!("{5}{R}{R}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
