//! Rivendell — (no cost) — Legendary Land
//! Oracle: Rivendell enters tapped unless you control a legendary creature.
//! Oracle: {T}: Add {U}.
//! Oracle: {1}{U}, {T}: Scry 2. Activate only if you control a legendary creature.
//! Set: LTR #259 — The Lord of the Rings: Tales of Middle-earth | Scryfall ID: bacd500c-1389-4314-a53e-0ad510d6fb79 | Oracle ID: 2550099d-b3e2-4eb6-9f36-0fc412828ca6
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 925,
    oracle_id: "2550099d-b3e2-4eb6-9f36-0fc412828ca6",
    scryfall_id: "bacd500c-1389-4314-a53e-0ad510d6fb79",
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    faces: &[
    face! {
        name: "Rivendell",
        types: TypeSet::LAND,
        supertypes: SupertypeSet::LEGENDARY,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
