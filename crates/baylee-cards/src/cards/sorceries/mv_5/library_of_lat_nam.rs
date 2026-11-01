//! Library of Lat-Nam — {4}{U} — Sorcery
//! Oracle: An opponent chooses one —
//! Oracle: • You draw three cards at the beginning of the next turn's upkeep.
//! Oracle: • You search your library for a card, put that card into your hand, then shuffle.
//! Set: 6ED #78 — Classic Sixth Edition | Scryfall ID: 9ea40438-90ca-47ff-9805-df130d47ae48 | Oracle ID: 67f68a0f-bd3d-47ba-ad99-95c1a2acd170
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LIBRARY_OF_LAT_NAM,
    oracle_id = "67f68a0f-bd3d-47ba-ad99-95c1a2acd170",
    scryfall_id = "9ea40438-90ca-47ff-9805-df130d47ae48",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Library of Lat-Nam",
        mana_cost = mana!("{4}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
