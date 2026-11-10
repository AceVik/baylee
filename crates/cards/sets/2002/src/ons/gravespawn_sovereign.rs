//! Gravespawn Sovereign — {4}{B}{B} — Creature — Zombie
//! Oracle: Tap five untapped Zombies you control: Put target creature card from a graveyard onto the battlefield under your control.
//! Set: MIC #120 — Midnight Hunt Commander | Scryfall ID: 58d36326-882e-46bb-a00c-5ea0f6515f65 | Oracle ID: 4235c99b-50de-4190-8938-3ce5ec5c5248
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GRAVESPAWN_SOVEREIGN,
    oracle_id = "4235c99b-50de-4190-8938-3ce5ec5c5248",
    scryfall_id = "58d36326-882e-46bb-a00c-5ea0f6515f65",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Gravespawn Sovereign",
        mana_cost = mana!("{4}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
