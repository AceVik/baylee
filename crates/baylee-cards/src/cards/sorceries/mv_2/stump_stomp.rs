//! Stump Stomp // Burnwillow Clearing — {1}{R/G} — Sorcery // Land
//! Set: MH3 #259 — Modern Horizons 3 | Scryfall ID: 49974246-0a3b-4ec9-b5ea-2a89df9bb0b5 | Oracle ID: eb7b1284-0b2c-4b6a-a389-b2b932838083
//! Face: Stump Stomp — {1}{R/G} — Sorcery
//! Face: Burnwillow Clearing —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 1098,
    oracle_id: "eb7b1284-0b2c-4b6a-a389-b2b932838083",
    scryfall_id: "49974246-0a3b-4ec9-b5ea-2a89df9bb0b5",
    color_identity: ColorSet::from_slice(&[Color::Green, Color::Red]),
    faces: &[
    face! {
        name: "Stump Stomp",
        mana_cost: baylee_core::mana!("{1}{R/G}"),
        types: TypeSet::SORCERY,
    },
    face! {
        name: "Burnwillow Clearing",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
