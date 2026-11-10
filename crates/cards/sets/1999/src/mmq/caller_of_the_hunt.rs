//! Caller of the Hunt — {2}{G} — Creature — Human
//! Oracle: As an additional cost to cast this spell, choose a creature type.
//! Oracle: Caller of the Hunt's power and toughness are each equal to the number of creatures of the chosen type on the battlefield.
//! Set: MMQ #233 — Mercadian Masques | Scryfall ID: c0e8e1cf-0a47-4ce4-889a-091229d0e466 | Oracle ID: 2ad60059-7a29-40a8-b595-3932f1222d33
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CALLER_OF_THE_HUNT,
    oracle_id = "2ad60059-7a29-40a8-b595-3932f1222d33",
    scryfall_id = "c0e8e1cf-0a47-4ce4-889a-091229d0e466",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Caller of the Hunt",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN],
        power = Some(0),
        toughness = Some(0),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
