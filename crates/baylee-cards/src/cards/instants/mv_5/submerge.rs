//! Submerge — {4}{U} — Instant
//! Oracle: If an opponent controls a Forest and you control an Island, you may cast this spell without paying its mana cost.
//! Oracle: Put target creature on top of its owner's library.
//! Set: NEM #48 — Nemesis | Scryfall ID: d2741fe4-37fe-427f-ae85-5107991d4eee | Oracle ID: 99427ebe-c00d-4206-84ca-9764f6e952c6
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SUBMERGE,
    oracle_id = "99427ebe-c00d-4206-84ca-9764f6e952c6",
    scryfall_id = "d2741fe4-37fe-427f-ae85-5107991d4eee",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Submerge",
        mana_cost = mana!("{4}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
