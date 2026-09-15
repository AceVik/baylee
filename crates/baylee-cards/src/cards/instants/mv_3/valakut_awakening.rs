//! Valakut Awakening // Valakut Stoneforge — {2}{R} — Instant // Land
//! Oracle: Put any number of cards from your hand on the bottom of your library, then draw that many cards plus one.
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {R}.
//! Set: ZNR #174 — Zendikar Rising | Scryfall ID: 228e551e-023a-4c9a-8f32-58dae6ffdf7f | Oracle ID: ff0ab867-b710-4b1a-baed-95fc3cf68f79
//! Face: Valakut Awakening — {2}{R} — Instant
//! Face: Valakut Stoneforge —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = 1274,
    oracle_id = "ff0ab867-b710-4b1a-baed-95fc3cf68f79",
    scryfall_id = "228e551e-023a-4c9a-8f32-58dae6ffdf7f",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[
        face!(
            name = "Valakut Awakening",
            mana_cost = mana!("{2}{R}"),
            types = TypeSet::INSTANT,
        ),
        face!(name = "Valakut Stoneforge", types = TypeSet::LAND,),
    ],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
