//! Westvale Abbey // Ormendahl, Profane Prince — (no cost) — Land // Legendary Creature — Demon
//! Set: INR #287 — Innistrad Remastered | Scryfall ID: 5fbc6091-a161-45b0-9932-543b569caaee | Oracle ID: 04eeb9ad-5c59-411b-8809-db8349838588
//! Face: Westvale Abbey —  — Land
//! Face: Ormendahl, Profane Prince —  — Legendary Creature — Demon
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 1314,
    oracle_id: "04eeb9ad-5c59-411b-8809-db8349838588",
    scryfall_id: "5fbc6091-a161-45b0-9932-543b569caaee",
    color_identity: ColorSet::from_slice(&[Color::Black]),
    faces: &[
    face! {
        name: "Westvale Abbey",
        types: TypeSet::LAND,
    },
    face! {
        name: "Ormendahl, Profane Prince",
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[subtypes::creature::DEMON],
        power: Some(9),
        toughness: Some(7),
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
