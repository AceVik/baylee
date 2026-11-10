//! Chaotic Strike — {1}{R} — Instant
//! Oracle: Cast this spell only during combat after blockers are declared.
//! Oracle: Flip a coin. If you win the flip, target creature gets +1/+1 until end of turn.
//! Oracle: Draw a card.
//! Set: INV #140 — Invasion | Scryfall ID: 061df8e4-6947-4bbb-9fe7-52ca4fd95d65 | Oracle ID: 1f0b3549-2d24-49e8-85bc-55e3a5e5f6b8
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CHAOTIC_STRIKE,
    oracle_id = "1f0b3549-2d24-49e8-85bc-55e3a5e5f6b8",
    scryfall_id = "061df8e4-6947-4bbb-9fe7-52ca4fd95d65",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Chaotic Strike",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
