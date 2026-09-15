//! Skyclave Cleric // Skyclave Basilica — {1}{W} — Creature — Kor Cleric // Land
//! Oracle: When this creature enters, you gain 2 life.
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {W}.
//! Set: ZNR #40 — Zendikar Rising | Scryfall ID: 014027c4-7f9d-4096-b308-ea4be574c0d4 | Oracle ID: da9e3910-9a1c-43a9-9138-ca971b2bccae
//! Face: Skyclave Cleric — {1}{W} — Creature — Kor Cleric
//! Face: Skyclave Basilica —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = 20552,
    oracle_id = "da9e3910-9a1c-43a9-9138-ca971b2bccae",
    scryfall_id = "014027c4-7f9d-4096-b308-ea4be574c0d4",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[
        face!(
            name = "Skyclave Cleric",
            mana_cost = mana!("{1}{W}"),
            types = TypeSet::CREATURE,
            subtypes = &[subtypes::creature::KOR, subtypes::creature::CLERIC],
            power = Some(1),
            toughness = Some(3),
        ),
        face!(name = "Skyclave Basilica", types = TypeSet::LAND,),
    ],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
