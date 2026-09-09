//! Barad-dûr — (no cost) — Legendary Land
//! Oracle: Barad-dûr enters tapped unless you control a legendary creature.
//! Oracle: {T}: Add {B}.
//! Oracle: {X}{X}{B}, {T}: Amass Orcs X. Activate only if a creature died this turn.
//! Set: LTR #253 — The Lord of the Rings: Tales of Middle-earth | Scryfall ID: eb5038af-06b0-401e-8dea-a1a8483788ae | Oracle ID: 88159872-d37d-4847-b048-e4a9af6437bd
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 258,
    oracle_id: "88159872-d37d-4847-b048-e4a9af6437bd",
    scryfall_id: "eb5038af-06b0-401e-8dea-a1a8483788ae",
    color_identity: ColorSet::from_slice(&[Color::Black]),
    faces: &[
    face! {
        name: "Barad-dûr",
        types: TypeSet::LAND,
        supertypes: SupertypeSet::LEGENDARY,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
