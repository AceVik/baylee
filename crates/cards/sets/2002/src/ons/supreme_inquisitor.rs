//! Supreme Inquisitor — {3}{U}{U} — Creature — Human Wizard
//! Oracle: Tap five untapped Wizards you control: Search target player's library for up to five cards and exile them. Then that player shuffles.
//! Set: ONS #117 — Onslaught | Scryfall ID: 867de3d2-2178-4931-823e-ff439e1a45ea | Oracle ID: 4460962e-c9cd-4cfd-b227-4278ee636206
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SUPREME_INQUISITOR,
    oracle_id = "4460962e-c9cd-4cfd-b227-4278ee636206",
    scryfall_id = "867de3d2-2178-4931-823e-ff439e1a45ea",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Supreme Inquisitor",
        mana_cost = mana!("{3}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
