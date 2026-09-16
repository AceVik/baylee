//! Open Communications — {U} — Sorcery
//! Oracle: Draw a card.
//! Oracle: Beam me up {2}{U} (You may cast this card from your graveyard for {2}{U} if you also return a creature you control to its owner's hand. Then exile this spell.)
//! Set: TRK #76 — Star Trek | Scryfall ID: 8eeec269-13f0-4923-9f13-e995acd73e00 | Oracle ID: b08c5b86-4f84-46a3-877f-bbf71ec17adb
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::OPEN_COMMUNICATIONS,
    oracle_id = "b08c5b86-4f84-46a3-877f-bbf71ec17adb",
    scryfall_id = "8eeec269-13f0-4923-9f13-e995acd73e00",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Open Communications",
        mana_cost = mana!("{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
