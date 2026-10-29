//! Degavolver — {1}{W} — Creature — Volver
//! Oracle: Kicker {1}{B} and/or {R} (You may pay an additional {1}{B} and/or {R} as you cast this spell.)
//! Oracle: If this creature was kicked with its {1}{B} kicker, it enters with two +1/+1 counters on it and with "Pay 3 life: Regenerate this creature."
//! Oracle: If this creature was kicked with its {R} kicker, it enters with a +1/+1 counter on it and with first strike.
//! Set: APC #6 — Apocalypse | Scryfall ID: 36a52c3a-2f58-4b4d-b3c6-f9a08e25c7de | Oracle ID: 9431c81f-7b71-4c5d-a63c-1b5aba03601b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DEGAVOLVER,
    oracle_id = "9431c81f-7b71-4c5d-a63c-1b5aba03601b",
    scryfall_id = "36a52c3a-2f58-4b4d-b3c6-f9a08e25c7de",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red, Color::White]),
    faces = &[face!(
        name = "Degavolver",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::VOLVER],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
