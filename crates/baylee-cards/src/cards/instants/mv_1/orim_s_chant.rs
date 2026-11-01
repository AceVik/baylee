//! Orim's Chant — {W} — Instant
//! Oracle: Kicker {W} (You may pay an additional {W} as you cast this spell.)
//! Oracle: Target player can't cast spells this turn. If this spell was kicked, creatures can't attack this turn.
//! Set: MH3 #265 — Modern Horizons 3 | Scryfall ID: ee241079-1e5a-4224-b9cb-4fd3e0da687c | Oracle ID: 26d3a818-7faf-443a-a6bd-9773edaedf17
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ORIM_S_CHANT,
    oracle_id = "26d3a818-7faf-443a-a6bd-9773edaedf17",
    scryfall_id = "ee241079-1e5a-4224-b9cb-4fd3e0da687c",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Orim's Chant",
        mana_cost = mana!("{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
