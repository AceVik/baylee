//! Vesuva — (no cost) — Land
//! Oracle: You may have this land enter tapped as a copy of any land on the battlefield.
//! Set: TSR #289 — Time Spiral Remastered | Scryfall ID: 0726f70a-c1c4-4edb-86fb-9be280d9ea73 | Oracle ID: 4001b868-ada1-43f4-92e2-27ab0e80c913

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::VESUVA,
    oracle_id = "4001b868-ada1-43f4-92e2-27ab0e80c913",
    scryfall_id = "0726f70a-c1c4-4edb-86fb-9be280d9ea73",
    faces = &[face!(name = "Vesuva", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[AbilityDef::CopyOnEnter {
        target: TargetSpec::Object(&Filter::LAND),
        mods: &[CopyMod::EnterTapped],
    }],
);
