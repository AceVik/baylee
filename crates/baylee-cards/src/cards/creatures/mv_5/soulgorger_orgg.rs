//! Soulgorger Orgg — {3}{R}{R} — Creature — Nightmare Orgg
//! Oracle: Trample
//! Oracle: When this creature enters, you lose all but 1 life.
//! Oracle: When this creature leaves the battlefield, you gain life equal to the life you lost when it entered.
//! Set: JUD #99 — Judgment | Scryfall ID: 8aef55b8-5813-4aff-a35d-4b3cbd4a9ffb | Oracle ID: cbbcd3a3-6ddd-4511-a352-3055d0102e7c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SOULGORGER_ORGG,
    oracle_id = "cbbcd3a3-6ddd-4511-a352-3055d0102e7c",
    scryfall_id = "8aef55b8-5813-4aff-a35d-4b3cbd4a9ffb",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Soulgorger Orgg",
        mana_cost = mana!("{3}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::NIGHTMARE, subtypes::creature::ORGG],
        power = Some(6),
        toughness = Some(6),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
