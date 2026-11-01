//! Dense Foliage — {2}{G} — Enchantment
//! Oracle: Creatures can't be the targets of spells.
//! Set: 6ED #221 — Classic Sixth Edition | Scryfall ID: f6232f04-cd4c-4889-97dc-cccc4a09f9c4 | Oracle ID: fb1369aa-25a2-4e2a-9cbe-6cfe7e238b1b
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DENSE_FOLIAGE,
    oracle_id = "fb1369aa-25a2-4e2a-9cbe-6cfe7e238b1b",
    scryfall_id = "f6232f04-cd4c-4889-97dc-cccc4a09f9c4",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Dense Foliage",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[static_ability!(
        Filter::CREATURE,
        Modifier::CantBeTargetedBy(&Filter::InZone(ZoneRef::Stack))
    )],
);
