//! Hydromorph Guardian — {2}{U} — Creature — Elemental
//! Oracle: {U}, Sacrifice this creature: Counter target spell that targets a creature you control.
//! Set: TOR #39 — Torment | Scryfall ID: 40e4ace0-421d-4e8e-ab85-8f74757e99c7 | Oracle ID: c426c2b0-82fa-4ea0-bbdb-3a40e60c495e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HYDROMORPH_GUARDIAN,
    oracle_id = "c426c2b0-82fa-4ea0-bbdb-3a40e60c495e",
    scryfall_id = "40e4ace0-421d-4e8e-ab85-8f74757e99c7",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Hydromorph Guardian",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELEMENTAL],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
