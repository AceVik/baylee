//! Brass's Tunnel-Grinder // Tecutlan, the Searing Rift — {2}{R} — Legendary Artifact // Legendary Land — Cave
//! Set: LCI #135 — The Lost Caverns of Ixalan | Scryfall ID: d61d8895-7f2e-4c77-951f-4f1a49e96f57 | Oracle ID: af1553eb-4f9f-4335-9078-56649bd8d8fc
//! Face: Brass's Tunnel-Grinder — {2}{R} — Legendary Artifact
//! Face: Tecutlan, the Searing Rift —  — Legendary Land — Cave
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 309,
    oracle_id: "af1553eb-4f9f-4335-9078-56649bd8d8fc",
    scryfall_id: "d61d8895-7f2e-4c77-951f-4f1a49e96f57",
    color_identity: ColorSet::from_slice(&[Color::Red]),
    faces: &[
    face! {
        name: "Brass's Tunnel-Grinder",
        mana_cost: baylee_core::mana!("{2}{R}"),
        types: TypeSet::ARTIFACT,
        supertypes: SupertypeSet::LEGENDARY,
    },
    face! {
        name: "Tecutlan, the Searing Rift",
        types: TypeSet::LAND,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[subtypes::land::CAVE],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
