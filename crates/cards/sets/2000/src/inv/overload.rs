//! Overload — {R} — Instant
//! Oracle: Kicker {2} (You may pay an additional {2} as you cast this spell.)
//! Oracle: Destroy target artifact if its mana value is 2 or less. If this spell was kicked, destroy that artifact if its mana value is 5 or less instead.
//! Set: INV #157 — Invasion | Scryfall ID: c91fca91-7296-422e-b251-d571b710ff71 | Oracle ID: 07159efc-c69f-4164-a8ca-9da641dbf702
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::OVERLOAD,
    oracle_id = "07159efc-c69f-4164-a8ca-9da641dbf702",
    scryfall_id = "c91fca91-7296-422e-b251-d571b710ff71",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Overload",
        mana_cost = mana!("{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
