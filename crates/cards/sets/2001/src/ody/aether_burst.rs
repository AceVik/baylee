//! Aether Burst — {1}{U} — Instant
//! Oracle: Return up to X target creatures to their owners' hands, where X is one plus the number of cards named Aether Burst in all graveyards as you cast this spell.
//! Set: ODY #60 — Odyssey | Scryfall ID: 3d09ea4a-0d3a-40b8-be0f-a31693099c4f | Oracle ID: 1e33f39b-a61a-4a09-a541-16cc1bd53d02
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::AETHER_BURST,
    oracle_id = "1e33f39b-a61a-4a09-a541-16cc1bd53d02",
    scryfall_id = "3d09ea4a-0d3a-40b8-be0f-a31693099c4f",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Aether Burst",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
