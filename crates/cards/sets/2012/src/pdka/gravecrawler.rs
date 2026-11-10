//! Gravecrawler — {B} — Creature — Zombie
//! Oracle: This creature can't block.
//! Oracle: You may cast this card from your graveyard as long as you control a Zombie.
//! Set: TDC #93 — Tarkir: Dragonstorm Commander | Scryfall ID: 6987d609-ba0f-42bf-9b61-bdfb943c03b5 | Oracle ID: 09ff28b1-b6c9-48e6-b12e-2f0e644f709f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GRAVECRAWLER,
    oracle_id = "09ff28b1-b6c9-48e6-b12e-2f0e644f709f",
    scryfall_id = "6987d609-ba0f-42bf-9b61-bdfb943c03b5",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Gravecrawler",
        mana_cost = mana!("{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE],
        power = Some(2),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
