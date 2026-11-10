//! Death Pulse — {2}{B}{B} — Instant
//! Oracle: Target creature gets -4/-4 until end of turn.
//! Oracle: Cycling {1}{B}{B} ({1}{B}{B}, Discard this card: Draw a card.)
//! Oracle: When you cycle this card, you may have target creature get -1/-1 until end of turn.
//! Set: ONS #137 — Onslaught | Scryfall ID: 524fd470-e535-47ea-98a0-6187e429dfe1 | Oracle ID: 2227bab2-36a0-47f1-a797-7b82323eca9d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DEATH_PULSE,
    oracle_id = "2227bab2-36a0-47f1-a797-7b82323eca9d",
    scryfall_id = "524fd470-e535-47ea-98a0-6187e429dfe1",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Death Pulse",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
