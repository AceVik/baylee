//! Fiery Temper — {1}{R}{R} — Instant
//! Oracle: Fiery Temper deals 3 damage to any target.
//! Oracle: Madness {R} (If you discard this card, discard it into exile. When you do, cast it for its madness cost or put it into your graveyard.)
//! Set: INR #154 — Innistrad Remastered | Scryfall ID: 1d3a3a1d-c393-4a57-8a0e-5907d1722331 | Oracle ID: f07bd49d-8e71-4d56-be2a-638514011318
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FIERY_TEMPER,
    oracle_id = "f07bd49d-8e71-4d56-be2a-638514011318",
    scryfall_id = "1d3a3a1d-c393-4a57-8a0e-5907d1722331",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Fiery Temper",
        mana_cost = mana!("{1}{R}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
