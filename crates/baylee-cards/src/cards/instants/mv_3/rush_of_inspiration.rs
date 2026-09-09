//! Rush of Inspiration // Crackling Falls — {1}{U/R}{U/R} — Instant // Land
//! Set: MH3 #257 — Modern Horizons 3 | Scryfall ID: 70a25a3a-c12a-49d3-8a91-a108dfa9d3c5 | Oracle ID: bbd569cc-bc21-46df-b8eb-5b5bcd8fe762
//! Face: Rush of Inspiration — {1}{U/R}{U/R} — Instant
//! Face: Crackling Falls —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 946,
    oracle_id: "bbd569cc-bc21-46df-b8eb-5b5bcd8fe762",
    scryfall_id: "70a25a3a-c12a-49d3-8a91-a108dfa9d3c5",
    color_identity: ColorSet::from_slice(&[Color::Red, Color::Blue]),
    faces: &[
    face! {
        name: "Rush of Inspiration",
        mana_cost: baylee_core::mana!("{1}{U/R}{U/R}"),
        types: TypeSet::INSTANT,
    },
    face! {
        name: "Crackling Falls",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
