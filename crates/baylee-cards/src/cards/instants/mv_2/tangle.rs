//! Tangle — {1}{G} — Instant
//! Oracle: Prevent all combat damage that would be dealt this turn.
//! Oracle: Each attacking creature doesn't untap during its controller's next untap step.
//! Set: VMA #237 — Vintage Masters | Scryfall ID: 635ca3c1-d544-44fc-a178-3bfe4fe69551 | Oracle ID: f627e125-15af-4e53-b34e-82b60e4ec87b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TANGLE,
    oracle_id = "f627e125-15af-4e53-b34e-82b60e4ec87b",
    scryfall_id = "635ca3c1-d544-44fc-a178-3bfe4fe69551",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Tangle",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
