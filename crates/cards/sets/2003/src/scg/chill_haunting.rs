//! Chill Haunting — {1}{B} — Instant
//! Oracle: As an additional cost to cast this spell, exile X creature cards from your graveyard.
//! Oracle: Target creature gets -X/-X until end of turn.
//! Set: SCG #60 — Scourge | Scryfall ID: 91035d03-2bf8-4e6b-945b-4dfbed0873ec | Oracle ID: ebe61471-e236-4ff8-9416-151970b5efd6
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CHILL_HAUNTING,
    oracle_id = "ebe61471-e236-4ff8-9416-151970b5efd6",
    scryfall_id = "91035d03-2bf8-4e6b-945b-4dfbed0873ec",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Chill Haunting",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
