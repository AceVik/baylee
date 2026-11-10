//! Golem-Skin Gauntlets — {1} — Artifact — Equipment
//! Oracle: Equipped creature gets +1/+0 for each Equipment attached to it.
//! Oracle: Equip {2} ({2}: Attach to target creature you control. Equip only as a sorcery.)
//! Set: 2XM #259 — Double Masters | Scryfall ID: ee198804-a247-46b9-be6e-71bbb5840401 | Oracle ID: 2a956224-8bdb-40d8-8405-2aa29eb228e0
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ATTACHED_EQUIPMENT: Filter = Filter::And(&[
    Filter::HasSubtype(subtypes::artifact::EQUIPMENT),
    Filter::AttachedToSource,
]);

card!(
    index = index::GOLEM_SKIN_GAUNTLETS,
    oracle_id = "2a956224-8bdb-40d8-8405-2aa29eb228e0",
    scryfall_id = "ee198804-a247-46b9-be6e-71bbb5840401",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Golem-Skin Gauntlets",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::EQUIPMENT],
    ),],
    abilities = &[
        equip!(cost!("{2}")),
        static_ability!(
            Filter::AttachedToBySource,
            Modifier::ModifyPTPerCount {
                filter: &ATTACHED_EQUIPMENT,
                p: 1,
                t: 0,
            },
        ),
    ],
);
