//! Carnival // Carnage — {B/R} — Instant // Sorcery
//! Oracle: Carnival deals 1 damage to target creature or planeswalker and 1 damage to that permanent's controller.
//! Oracle: Carnage deals 3 damage to target opponent. That player discards two cards.
//! Set: RNA #222 — Ravnica Allegiance | Scryfall ID: d1dbc559-c78c-4675-9582-9c28f8151bc7 | Oracle ID: b996ae86-3bf6-4e84-8302-6ec6edaa6ff0
//! Face: Carnival — {B/R} — Instant
//! Face: Carnage — {2}{B}{R} — Sorcery
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CARNIVAL,
    oracle_id = "b996ae86-3bf6-4e84-8302-6ec6edaa6ff0",
    scryfall_id = "d1dbc559-c78c-4675-9582-9c28f8151bc7",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    faces = &[
        face!(
            name = "Carnival",
            mana_cost = mana!("{B/R}"),
            types = TypeSet::INSTANT,
        ),
        face!(
            name = "Carnage",
            mana_cost = mana!("{2}{B}{R}"),
            types = TypeSet::SORCERY,
        ),
    ],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
