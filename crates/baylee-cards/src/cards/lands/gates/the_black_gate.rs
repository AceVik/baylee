//! The Black Gate — (no cost) — Legendary Land — Gate
//! Oracle: As The Black Gate enters, you may pay 3 life. If you don't, it enters tapped.
//! Oracle: {T}: Add {B}.
//! Oracle: {1}{B}, {T}: Choose a player with the most life or tied for most life. Target creature can't be blocked by creatures that player controls this turn.
//! Set: LTC #80 — Tales of Middle-earth Commander | Scryfall ID: 46418186-c215-47c4-9d0a-d15a1d8ca613 | Oracle ID: 40eb9904-dea3-47cf-963a-04821f98ba64
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 1168,
    oracle_id: "40eb9904-dea3-47cf-963a-04821f98ba64",
    scryfall_id: "46418186-c215-47c4-9d0a-d15a1d8ca613",
    color_identity: ColorSet::from_slice(&[Color::Black]),
    faces: &[
    face! {
        name: "The Black Gate",
        types: TypeSet::LAND,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[subtypes::land::GATE],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
