//! Oracle's Attendants — {3}{W} — Creature — Human Soldier
//! Oracle: {T}: All damage that would be dealt to target creature this turn by a source of your choice is dealt to this creature instead.
//! Set: 9ED #30 — Ninth Edition | Scryfall ID: 11a180a4-bffc-4f4f-ae34-3afa4a565960 | Oracle ID: f4bc6674-8586-4dd9-ad3a-87ba704fda7a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ORACLE_S_ATTENDANTS,
    oracle_id = "f4bc6674-8586-4dd9-ad3a-87ba704fda7a",
    scryfall_id = "11a180a4-bffc-4f4f-ae34-3afa4a565960",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Oracle's Attendants",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(1),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
