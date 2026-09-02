//! Hostile Hostel // Creeping Inn — (no cost) — Land // Artifact Creature — Horror Construct
//! Set: MID #264 — Innistrad: Midnight Hunt | Scryfall ID: ac83c27f-55d6-4e5a-93a4-febb0c183289 | Oracle ID: 1b340f71-502f-48e9-85ed-9af62f356115
//! Face: Hostile Hostel —  — Land
//! Face: Creeping Inn —  — Artifact Creature — Horror Construct
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 639,
    oracle_id: "1b340f71-502f-48e9-85ed-9af62f356115",
    scryfall_id: "ac83c27f-55d6-4e5a-93a4-febb0c183289",
    color_identity: ColorSet::from_slice(&[Color::Black]),
    faces: &[
    face! {
        name: "Hostile Hostel",
        types: TypeSet::LAND,
    },
    face! {
        name: "Creeping Inn",
        types: TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes: &[subtypes::creature::HORROR, subtypes::creature::CONSTRUCT],
        power: Some(3),
        toughness: Some(7),
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
