//! Soulless One — {3}{B} — Creature — Zombie Avatar
//! Oracle: Soulless One's power and toughness are each equal to the number of Zombies on the battlefield plus the number of Zombie cards in all graveyards.
//! Set: HOP #41 — Planechase | Scryfall ID: 410a214b-09c4-49bd-a461-3330d0249ae5 | Oracle ID: 27d98b94-a8a1-442b-8940-4ffab51b5164
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SOULLESS_ONE,
    oracle_id = "27d98b94-a8a1-442b-8940-4ffab51b5164",
    scryfall_id = "410a214b-09c4-49bd-a461-3330d0249ae5",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Soulless One",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE, subtypes::creature::AVATAR],
        power = Some(0),
        toughness = Some(0),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
