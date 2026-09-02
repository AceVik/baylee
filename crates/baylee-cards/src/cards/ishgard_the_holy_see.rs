//! Ishgard, the Holy See // Faith & Grief — (no cost) — Land — Town // Sorcery — Adventure
//! Set: FIN #283 — Final Fantasy | Scryfall ID: 068bc755-9d3d-430b-abc5-c775a5415bf9 | Oracle ID: 4f4358cb-59df-46d9-be27-69929f5a615c
//! Face: Ishgard, the Holy See —  — Land — Town
//! Face: Faith & Grief — {3}{W}{W} — Sorcery — Adventure
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 660,
    oracle_id: "4f4358cb-59df-46d9-be27-69929f5a615c",
    scryfall_id: "068bc755-9d3d-430b-abc5-c775a5415bf9",
    color_identity: ColorSet::from_slice(&[Color::White]),
    faces: &[
    face! {
        name: "Ishgard, the Holy See",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::TOWN],
    },
    face! {
        name: "Faith & Grief",
        mana_cost: baylee_core::mana!("{3}{W}{W}"),
        types: TypeSet::SORCERY,
        subtypes: &[subtypes::spell::ADVENTURE],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
