//! Inflame — {R} — Instant
//! Oracle: Inflame deals 2 damage to each creature dealt damage this turn.
//! Set: DST #64 — Darksteel | Scryfall ID: e1efad9a-2fcf-4045-8105-bf9f5e79d12c | Oracle ID: 8426bdb6-b002-4d27-9eaf-d7f8353aa7e8
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::INFLAME,
    oracle_id = "8426bdb6-b002-4d27-9eaf-d7f8353aa7e8",
    scryfall_id = "e1efad9a-2fcf-4045-8105-bf9f5e79d12c",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Inflame",
        mana_cost = mana!("{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
