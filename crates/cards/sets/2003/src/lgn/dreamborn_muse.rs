//! Dreamborn Muse — {2}{U}{U} — Creature — Spirit
//! Oracle: At the beginning of each player's upkeep, that player mills X cards, where X is the number of cards in their hand.
//! Set: CM2 #41 — Commander Anthology Volume II | Scryfall ID: 83ad2ba9-ab54-4abc-805d-dc9fe31bac7f | Oracle ID: 68d7f338-c6e7-4372-b41c-901e39e08bde
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DREAMBORN_MUSE,
    oracle_id = "68d7f338-c6e7-4372-b41c-901e39e08bde",
    scryfall_id = "83ad2ba9-ab54-4abc-805d-dc9fe31bac7f",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Dreamborn Muse",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPIRIT],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
