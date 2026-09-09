//! Dromar's Cavern — (no cost) — Land — Lair
//! Oracle: When this land enters, sacrifice it unless you return a non-Lair land you control to its owner's hand.
//! Oracle: {T}: Add {W}, {U}, or {B}.
//! Set: DMR #246 — Dominaria Remastered | Scryfall ID: 10380662-88bd-4bc5-b8c3-14af425fc5a5 | Oracle ID: d8b57707-796d-4488-8f91-65bb75bc6281
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 442,
    oracle_id: "d8b57707-796d-4488-8f91-65bb75bc6281",
    scryfall_id: "10380662-88bd-4bc5-b8c3-14af425fc5a5",
    color_identity: ColorSet::from_slice(&[Color::Black, Color::Blue, Color::White]),
    faces: &[
    face! {
        name: "Dromar's Cavern",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::LAIR],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
