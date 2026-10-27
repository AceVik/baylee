//! Samite Elder — {2}{W} — Creature — Human Cleric
//! Oracle: {T}: Choose target permanent you control. Creatures you control gain protection from each of that permanent's colors until end of turn.
//! Set: PLS #14 — Planeshift | Scryfall ID: b3c5dccc-2a48-4dcc-a796-fa6fdc11a14e | Oracle ID: 8feb27aa-1f62-46e5-ba20-72c712de0862
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SAMITE_ELDER,
    oracle_id = "8feb27aa-1f62-46e5-ba20-72c712de0862",
    scryfall_id = "b3c5dccc-2a48-4dcc-a796-fa6fdc11a14e",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Samite Elder",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
