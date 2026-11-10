//! Snag — {3}{G} — Instant
//! Oracle: You may discard a Forest card rather than pay this spell's mana cost.
//! Oracle: Prevent all combat damage that would be dealt by unblocked creatures this turn.
//! Set: PCY #124 — Prophecy | Scryfall ID: 7401df8b-23e7-4485-9ed4-70118a66feed | Oracle ID: feb5e9a9-500a-4f20-881d-c97d052953ce
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SNAG,
    oracle_id = "feb5e9a9-500a-4f20-881d-c97d052953ce",
    scryfall_id = "7401df8b-23e7-4485-9ed4-70118a66feed",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Snag",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
