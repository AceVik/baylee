//! Bonethorn Valesk — {4}{R} — Creature — Beast
//! Oracle: Whenever a permanent is turned face up, this creature deals 1 damage to any target.
//! Set: SCG #82 — Scourge | Scryfall ID: 297d7326-ad03-464d-97e2-443042d48f92 | Oracle ID: 32bb6d53-3b9e-4f13-a17e-46b564afd366
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BONETHORN_VALESK,
    oracle_id = "32bb6d53-3b9e-4f13-a17e-46b564afd366",
    scryfall_id = "297d7326-ad03-464d-97e2-443042d48f92",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Bonethorn Valesk",
        mana_cost = mana!("{4}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(4),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
