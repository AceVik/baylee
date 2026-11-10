//! Cephalid Snitch — {1}{U} — Creature — Octopus Wizard
//! Oracle: Sacrifice this creature: Target creature loses protection from black until end of turn.
//! Set: TOR #30 — Torment | Scryfall ID: 33d989b2-0198-4e5b-8aad-ee939191dd28 | Oracle ID: 15093579-8258-41d2-9738-aaa7e3824c40
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CEPHALID_SNITCH,
    oracle_id = "15093579-8258-41d2-9738-aaa7e3824c40",
    scryfall_id = "33d989b2-0198-4e5b-8aad-ee939191dd28",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Cephalid Snitch",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::OCTOPUS, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
