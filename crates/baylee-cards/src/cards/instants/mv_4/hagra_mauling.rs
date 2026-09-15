//! Hagra Mauling // Hagra Broodpit — {2}{B}{B} — Instant // Land
//! Oracle: This spell costs {1} less to cast if an opponent controls no basic lands.
//! Oracle: Destroy target creature.
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {B}.
//! Set: ZNR #106 — Zendikar Rising | Scryfall ID: 7c04c734-354d-4925-8161-7052110951df | Oracle ID: 37783ce6-af58-4ef6-8ab4-587079970307
//! Face: Hagra Mauling — {2}{B}{B} — Instant
//! Face: Hagra Broodpit —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = 593,
    oracle_id = "37783ce6-af58-4ef6-8ab4-587079970307",
    scryfall_id = "7c04c734-354d-4925-8161-7052110951df",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[
        face!(
            name = "Hagra Mauling",
            mana_cost = mana!("{2}{B}{B}"),
            types = TypeSet::INSTANT,
        ),
        face!(name = "Hagra Broodpit", types = TypeSet::LAND,),
    ],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
