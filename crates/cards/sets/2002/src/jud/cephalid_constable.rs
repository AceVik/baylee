//! Cephalid Constable — {1}{U}{U} — Creature — Octopus Wizard
//! Oracle: Whenever this creature deals combat damage to a player, return up to that many target permanents that player controls to their owner's hand.
//! Set: 10E #72 — Tenth Edition | Scryfall ID: 82c5272d-df5b-4c21-9bff-4d6245b8d2a6 | Oracle ID: a5236bf7-c84f-4c64-bf6b-3a89c05e161a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CEPHALID_CONSTABLE,
    oracle_id = "a5236bf7-c84f-4c64-bf6b-3a89c05e161a",
    scryfall_id = "82c5272d-df5b-4c21-9bff-4d6245b8d2a6",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Cephalid Constable",
        mana_cost = mana!("{1}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::OCTOPUS, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
