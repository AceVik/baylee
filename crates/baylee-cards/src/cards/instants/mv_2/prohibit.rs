//! Prohibit — {1}{U} — Instant
//! Oracle: Kicker {2} (You may pay an additional {2} as you cast this spell.)
//! Oracle: Counter target spell if its mana value is 2 or less. If this spell was kicked, counter that spell if its mana value is 4 or less instead.
//! Set: MH1 #64 — Modern Horizons | Scryfall ID: 0ae544bf-7229-4b82-99ad-32c3af36e30f | Oracle ID: d95f1797-56f9-41be-a5fe-8398961f4b8b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PROHIBIT,
    oracle_id = "d95f1797-56f9-41be-a5fe-8398961f4b8b",
    scryfall_id = "0ae544bf-7229-4b82-99ad-32c3af36e30f",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Prohibit",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
