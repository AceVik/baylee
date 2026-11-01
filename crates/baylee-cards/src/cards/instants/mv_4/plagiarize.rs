//! Plagiarize — {3}{U} — Instant
//! Oracle: Until end of turn, if target player would draw a card, instead that player skips that draw and you draw a card.
//! Set: 10E #97 — Tenth Edition | Scryfall ID: d59ae2f8-1e19-495a-86a9-8391e1e14ac5 | Oracle ID: ba4f9450-d468-4686-9f48-00fa08c6ac52
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PLAGIARIZE,
    oracle_id = "ba4f9450-d468-4686-9f48-00fa08c6ac52",
    scryfall_id = "d59ae2f8-1e19-495a-86a9-8391e1e14ac5",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Plagiarize",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
