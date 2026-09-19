//! Crop Rotation — {G} — Instant
//! Oracle: As an additional cost to cast this spell, sacrifice a land.
//! Oracle: Search your library for a land card, put that card onto the battlefield, then shuffle.
//! Set: DMR #154 — Dominaria Remastered | Scryfall ID: 523414cb-f8db-407a-808a-01454e03d8b9 | Oracle ID: 28b46183-c62f-47b1-9fee-3ba148202cab
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CROP_ROTATION,
    oracle_id = "28b46183-c62f-47b1-9fee-3ba148202cab",
    scryfall_id = "523414cb-f8db-407a-808a-01454e03d8b9",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Crop Rotation",
        mana_cost = mana!("{G}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
