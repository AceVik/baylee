//! Black Knight — {B}{B} — Creature — Human Knight
//! Oracle: First strike (This creature deals combat damage before creatures without first strike.)
//! Oracle: Protection from white (This creature can't be blocked, targeted, dealt damage, or enchanted by anything white.)
//! Set: ME4 #71 — Masters Edition IV | Scryfall ID: db593d4f-d3c1-48ae-a55d-4b61d2cb5cc2 | Oracle ID: 9456c5b6-946d-403a-8ed0-dff9f921d98c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BLACK_KNIGHT,
    oracle_id = "9456c5b6-946d-403a-8ed0-dff9f921d98c",
    scryfall_id = "db593d4f-d3c1-48ae-a55d-4b61d2cb5cc2",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Black Knight",
        mana_cost = mana!("{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::KNIGHT],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
