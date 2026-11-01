//! Pinpoint Avalanche — {3}{R}{R} — Instant
//! Oracle: Pinpoint Avalanche deals 4 damage to target creature. The damage can't be prevented.
//! Set: ONS #221 — Onslaught | Scryfall ID: d5cf8876-4c7d-4779-9363-d0a58bb7d851 | Oracle ID: d619cce4-036a-4da2-ae64-f43fc095ebe9
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PINPOINT_AVALANCHE,
    oracle_id = "d619cce4-036a-4da2-ae64-f43fc095ebe9",
    scryfall_id = "d5cf8876-4c7d-4779-9363-d0a58bb7d851",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Pinpoint Avalanche",
        mana_cost = mana!("{3}{R}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
