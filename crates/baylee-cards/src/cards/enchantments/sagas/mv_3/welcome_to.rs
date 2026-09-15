//! Welcome to . . . // Jurassic Park — {1}{G}{G} — Enchantment — Saga // Legendary Land
//! Oracle: (As this Saga enters and after your draw step, add a lore counter.)
//! Oracle: I — For each opponent, up to one target noncreature artifact they control becomes a 0/4 Wall artifact creature with defender for as long as you control this Saga.
//! Oracle: II — Create a 3/3 green Dinosaur creature token with trample. It gains haste until end of turn.
//! Oracle: III — Destroy all Walls. Exile this Saga, then return it to the battlefield transformed under your control.
//! Oracle: (Transforms from Welcome to ....)
//! Oracle: Each Dinosaur card in your graveyard has escape. The escape cost is equal to the card's mana cost plus exile three other cards from your graveyard. (You may cast cards from your graveyard for their escape cost.)
//! Oracle: {T}: Add {G} for each Dinosaur you control.
//! Set: REX #7 — Jurassic World Collection | Scryfall ID: 6d84e2d4-38bf-4d46-99a6-37c2dda66b16 | Oracle ID: a4b37d16-95b3-4143-a0b2-ad9f2aba91f8
//! Face: Welcome to . . . — {1}{G}{G} — Enchantment — Saga
//! Face: Jurassic Park —  — Legendary Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 1313,
    oracle_id: "a4b37d16-95b3-4143-a0b2-ad9f2aba91f8",
    scryfall_id: "6d84e2d4-38bf-4d46-99a6-37c2dda66b16",
    color_identity: ColorSet::from_slice(&[Color::Green]),
    faces: &[
    face! {
        name: "Welcome to . . .",
        mana_cost: mana!("{1}{G}{G}"),
        types: TypeSet::ENCHANTMENT,
        subtypes: &[subtypes::enchantment::SAGA],
    },
    face! {
        name: "Jurassic Park",
        types: TypeSet::LAND,
        supertypes: SupertypeSet::LEGENDARY,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
