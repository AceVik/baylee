//! Vampiric Tutor — {B} — Instant
//! Oracle: Search your library for a card, then shuffle and put that card on top. You lose 2 life.
//! Set: DMR #108 — Dominaria Remastered | Scryfall ID: 34a0203f-9cce-43a4-9cb7-8ce6647895cd | Oracle ID: ededbdae-d9dc-4206-9335-d7158f2d7700
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::VAMPIRIC_TUTOR,
    oracle_id = "ededbdae-d9dc-4206-9335-d7158f2d7700",
    scryfall_id = "34a0203f-9cce-43a4-9cb7-8ce6647895cd",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Vampiric Tutor",
        mana_cost = mana!("{B}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
