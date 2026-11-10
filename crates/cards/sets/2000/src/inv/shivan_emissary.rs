//! Shivan Emissary — {2}{R} — Creature — Human Wizard
//! Oracle: Kicker {1}{B} (You may pay an additional {1}{B} as you cast this spell.)
//! Oracle: When this creature enters, if it was kicked, destroy target nonblack creature. It can't be regenerated.
//! Set: INV #166 — Invasion | Scryfall ID: 945c596e-492e-4cf5-857c-4ddbbdd78485 | Oracle ID: 1b885600-def1-4b2c-a6cb-02df34d58a1b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SHIVAN_EMISSARY,
    oracle_id = "1b885600-def1-4b2c-a6cb-02df34d58a1b",
    scryfall_id = "945c596e-492e-4cf5-857c-4ddbbdd78485",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    faces = &[face!(
        name = "Shivan Emissary",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
