//! Vengeful Dreams — {W}{W} — Instant
//! Oracle: As an additional cost to cast this spell, discard X cards.
//! Oracle: Exile X target attacking creatures.
//! Set: TOR #21 — Torment | Scryfall ID: 48bfe8b2-cc98-4430-b576-085163bdf0b6 | Oracle ID: 55d01851-2669-4552-b580-088e445ac30c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::VENGEFUL_DREAMS,
    oracle_id = "55d01851-2669-4552-b580-088e445ac30c",
    scryfall_id = "48bfe8b2-cc98-4430-b576-085163bdf0b6",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Vengeful Dreams",
        mana_cost = mana!("{W}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
