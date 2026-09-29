//! Power Sink — {X}{U} — Instant
//! Oracle: Counter target spell unless its controller pays {X}. If that player doesn't, they tap all lands with mana abilities they control and lose all unspent mana.
//! Set: VMA #88 — Vintage Masters | Scryfall ID: 9a6f3ce5-d4a5-4d7b-a7f9-b249c1d88e8f | Oracle ID: 39412e6d-2837-4729-abf9-e64a5ba87e40
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::POWER_SINK,
    oracle_id = "39412e6d-2837-4729-abf9-e64a5ba87e40",
    scryfall_id = "9a6f3ce5-d4a5-4d7b-a7f9-b249c1d88e8f",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Power Sink",
        mana_cost = mana!("{X}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
