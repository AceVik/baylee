//! Archon of Emeria — {2}{W} — Creature — Archon
//! Oracle: Flying
//! Oracle: Each player can't cast more than one spell each turn.
//! Oracle: Nonbasic lands your opponents control enter tapped.
//! Set: ZNR #4 — Zendikar Rising | Scryfall ID: 228c1650-da3c-4099-91b6-18e3873c9cdb | Oracle ID: ceef2d5a-77ea-4e56-9806-fd1a2d5be400
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ARCHON_OF_EMERIA,
    oracle_id = "ceef2d5a-77ea-4e56-9806-fd1a2d5be400",
    scryfall_id = "228c1650-da3c-4099-91b6-18e3873c9cdb",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Archon of Emeria",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ARCHON],
        power = Some(2),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
