//! Kangee, Aerie Keeper — {2}{W}{U} — Legendary Creature — Bird Wizard
//! Oracle: Kicker {X}{2} (You may pay an additional {X}{2} as you cast this spell.)
//! Oracle: Flying
//! Oracle: When Kangee enters, if it was kicked, put X feather counters on it.
//! Oracle: Other Bird creatures get +1/+1 for each feather counter on Kangee.
//! Set: INV #253 — Invasion | Scryfall ID: 3afd7e8e-4fcc-4003-9791-7baf10ef1880 | Oracle ID: 555f9e75-8892-4c8e-9c5e-8ab14a589118
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KANGEE_AERIE_KEEPER,
    oracle_id = "555f9e75-8892-4c8e-9c5e-8ab14a589118",
    scryfall_id = "3afd7e8e-4fcc-4003-9791-7baf10ef1880",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Kangee, Aerie Keeper",
        mana_cost = mana!("{2}{W}{U}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::BIRD, subtypes::creature::WIZARD],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
