//! Force of Nature — {2}{G}{G}{G}{G} — Creature — Elemental
//! Oracle: Trample (This creature can deal excess combat damage to the player or planeswalker it's attacking.)
//! Oracle: At the beginning of your upkeep, this creature deals 8 damage to you unless you pay {G}{G}{G}{G}.
//! Set: ME4 #154 — Masters Edition IV | Scryfall ID: 2fa16a96-8e70-4ab5-926c-edafaf5f5a63 | Oracle ID: e3c4c27d-f263-4c69-a4fe-2928136ff68b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FORCE_OF_NATURE,
    oracle_id = "e3c4c27d-f263-4c69-a4fe-2928136ff68b",
    scryfall_id = "2fa16a96-8e70-4ab5-926c-edafaf5f5a63",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Force of Nature",
        mana_cost = mana!("{2}{G}{G}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELEMENTAL],
        power = Some(8),
        toughness = Some(8),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
