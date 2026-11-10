//! Vigorous Charge — {G} — Instant
//! Oracle: Kicker {W} (You may pay an additional {W} as you cast this spell.)
//! Oracle: Target creature gains trample until end of turn. Whenever that creature deals combat damage this turn, if this spell was kicked, you gain life equal to that damage.
//! Set: INV #222 — Invasion | Scryfall ID: af6f57ad-d370-4c81-8da0-c15d87725ab1 | Oracle ID: b8ce3f84-2053-4d16-ab3d-e552bd8d0e8c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::VIGOROUS_CHARGE,
    oracle_id = "b8ce3f84-2053-4d16-ab3d-e552bd8d0e8c",
    scryfall_id = "af6f57ad-d370-4c81-8da0-c15d87725ab1",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    faces = &[face!(
        name = "Vigorous Charge",
        mana_cost = mana!("{G}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
